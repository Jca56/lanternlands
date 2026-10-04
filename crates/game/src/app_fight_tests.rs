//! Fighting, dying and the grave, driven with no window.

use sim::MonsterKind;

use super::play::{pass, playing, pose_of, runs, screen_of};
use super::*;
use crate::scene::Placed;

/// The game with the player on open grass and one goblin living
/// `east` tiles to their east (as near that as there is ground to
/// stand on). The player is strong enough to win any fight with it.
pub(super) fn with_goblin(name: &str, east: i32) -> (Harness, Shell<Game>, Game) {
    let (mut h, mut shell, mut game) = playing(name);
    let (x, y) = game.sim.player.tile;
    let home = sim::player::place_to_stand(&game.sim.world, (x + east, y)).expect("somewhere to stand");
    assert!(game.sim.world.set_spawn(home.0, home.1, Some(MonsterKind::Goblin)));
    game.sim.settle_player();
    game.sim.skills.set_xp(Skill::Vitality, sim::skills::xp_for_level(20));
    game.sim.player.health = game.sim.max_health();
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game)
}

/// Where a monster's body is on screen: half a tile over its feet.
pub(super) fn body_of(game: &Game, monster: usize) -> Vec2 {
    game.camera.to_screen(game.monster_grounds()[monster] - Vec2::new(0.0, 0.5), game.view, game.scale)
}

/// Let time pass until `done`, at most `seconds`; gather what floated.
pub(super) fn until(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, seconds: f64, done: impl Fn(&Game) -> bool) -> Vec<String> {
    let mut floated = Vec::new();
    for _ in 0..(seconds / 0.05) as usize {
        if done(game) {
            break;
        }
        h.advance(0.05);
        turn(h, shell, game);
        for line in game.hud.floating() {
            if !floated.contains(&line.to_owned()) {
                floated.push(line.to_owned());
            }
        }
    }
    floated
}

fn drawn(game: &Game, sprite: crate::assets::SpriteId) -> Vec<Placed> {
    game.extras().into_iter().filter(|p| p.sprite == sprite).collect()
}

#[test]
fn a_goblin_out_of_sight_wanders_and_the_world_ticks_without_the_player() {
    let (mut h, mut shell, mut game) = with_goblin("fight_wander", 12);
    let home = game.sim.monsters[0].home;
    let mut stood = std::collections::HashSet::new();
    for _ in 0..600 {
        h.advance(0.1);
        turn(&mut h, &mut shell, &mut game);
        stood.insert(game.sim.monsters[0].tile);
    }
    assert!(stood.len() > 2, "it wandered: {stood:?}");
    assert!(stood.iter().all(|t| (t.0 - home.0).abs() <= 3 && (t.1 - home.1).abs() <= 3));
    assert!(!game.sim.monsters[0].angry && !game.sim.player.busy());
    // The game asks to be woken for the world's next tick, not every frame,
    // once the goblin is standing still.
    let waits: Vec<f64> = (0..20).filter_map(|_| { h.advance(0.05); turn(&mut h, &mut shell, &mut game).wake_after }).collect();
    assert!(waits.iter().any(|w| *w > 0.0), "between a goblin's steps the game sleeps until the tick: {waits:?}");
}

#[test]
fn the_pointer_finds_a_goblin_by_its_body_and_not_by_the_grass_beside_it() {
    let (_, _, game) = with_goblin("fight_aim", 3);
    let body = body_of(&game, 0);
    assert_eq!(game.monster_under(body), Some(0));
    let tile_px = game.camera.tile_px(game.scale);
    assert_eq!(game.monster_under(body + Vec2::new(tile_px, 0.0)), None);
    assert_eq!(game.monster_under(body - Vec2::new(0.0, tile_px * 1.2)), None);
}

#[test]
fn clicking_a_goblin_fights_it_until_it_falls() {
    let (mut h, mut shell, mut game) = with_goblin("fight_click", 3);
    let at = body_of(&game, 0);
    h.move_to(at);
    turn(&mut h, &mut shell, &mut game);
    h.press();
    turn(&mut h, &mut shell, &mut game);
    h.release();
    assert!(game.sim.player.busy(), "on their way to it, or at it already");
    let floated = until(&mut h, &mut shell, &mut game, 120.0, |game| !game.sim.monsters[0].alive());
    assert!(!game.sim.monsters[0].alive(), "the goblin fell");
    assert!(game.sim.skills.xp(Skill::Melee) >= 48, "experience for all twelve of its health");
    assert!(!floated.iter().any(|line| line.contains("xp")), "no experience floats over the fight: {floated:?}");
    assert!(floated.iter().any(|line| line.parse::<u32>().is_ok()), "the blows' numbers floated over it: {floated:?}");
    // The blow that killed it is still drawn as a strike.
    assert_eq!(pose_of(&game), Some(Pose::WorkB));
    // A dead goblin is not drawn, and the player goes back to standing,
    // ready to act the moment they are asked to.
    pass(&mut h, &mut shell, &mut game, 2.0);
    assert_eq!(pose_of(&game), Some(Pose::Idle));
    assert!(!game.sim.player.swung && game.phase == sim::TICK_SECONDS);
    let goblin = game.look.goblin;
    assert!(Pose::ALL.iter().all(|&pose| [Facing::Down, Facing::Up, Facing::Left, Facing::Right].iter().all(|&f| goblin.sprite(f, pose).is_none_or(|(id, _)| drawn(&game, id).is_empty()))));
    assert_eq!(game.sim.player.activity(), None);
}

#[test]
fn a_goblin_that_sees_the_player_comes_for_them_and_they_fight_back_unasked() {
    let (mut h, mut shell, mut game) = with_goblin("fight_sight", 3);
    until(&mut h, &mut shell, &mut game, 60.0, |game| !game.sim.monsters[0].alive());
    assert!(!game.sim.monsters[0].alive(), "nothing was clicked, and the goblin is dead");
    assert!(game.sim.skills.xp(Skill::Melee) > 0);
}

/// The pose a monster is drawn in.
fn monster_pose(game: &Game, monster: usize) -> Option<Pose> {
    let (sprites, facing) = (game.look.monster(game.sim.monsters[monster].kind), game.monster_facing[monster]);
    let extras = game.extras();
    Pose::ALL.into_iter().find(|&pose| sprites.sprite(facing, pose).is_some_and(|(id, flip)| extras.iter().any(|p| p.sprite == id && p.flip == flip)))
}

#[test]
fn in_a_fight_each_swings_once_for_each_blow_and_stands_ready_between() {
    let (mut h, mut shell, mut game) = with_goblin("fight_poses", 3);
    until(&mut h, &mut shell, &mut game, 30.0, |game| matches!(game.sim.player.activity(), Some(sim::Activity::Fighting { .. })));
    let (mut mine, mut its) = (Vec::new(), Vec::new());
    let (mut blows, mut swung, mut its_blows, mut its_last) = (0, game.sim.player.swung, 0, game.sim.monsters[0].swung_at);
    for _ in 0..400 {
        // Keep both alive to watch plenty of blows.
        game.sim.monsters[0].health = 12;
        game.sim.player.health = game.sim.max_health();
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        blows += (game.sim.player.swung && !swung) as usize;
        swung = game.sim.player.swung;
        its_blows += (game.sim.monsters[0].swung_at != its_last) as usize;
        its_last = game.sim.monsters[0].swung_at;
        mine.push(pose_of(&game).expect("the player is drawn"));
        its.push(monster_pose(&game, 0).expect("the goblin is drawn"));
        assert_ne!(game.sim.player.tile, game.sim.monsters[0].tile, "nobody stands on anybody");
    }
    assert!(blows >= 12 && its_blows >= 9, "{blows} blows given, {its_blows} taken");
    for (who, poses, blows) in [("the player", &mine, blows), ("the goblin", &its, its_blows)] {
        let (back, strikes) = (runs(poses, Pose::WorkA), runs(poses, Pose::WorkB));
        assert!(strikes.abs_diff(blows) <= 1, "{who}: {strikes} strikes drawn for {blows} blows");
        assert!(back.abs_diff(blows) <= 1, "{who}: {back} drawings back for {blows} blows");
        let swinging = poses.iter().filter(|p| matches!(p, Pose::WorkA | Pose::WorkB)).count();
        let ready = poses.iter().filter(|p| **p == Pose::Idle).count();
        assert!(ready > swinging, "{who} stands ready ({ready} frames) more than they swing ({swinging})");
        assert_eq!(ready + swinging, poses.len(), "{who} does nothing else");
    }
    // Side by side on a row, they face each other.
    let (p, m) = (game.sim.player.tile, game.sim.monsters[0].tile);
    if p.1 == m.1 {
        let toward = if m.0 > p.0 { (Facing::Right, Facing::Left) } else { (Facing::Left, Facing::Right) };
        assert_eq!((game.facing, game.monster_facing[0]), toward);
    }
}

#[test]
fn clicked_from_any_distance_they_fight_it_from_beside_it_never_from_on_top() {
    for (n, east) in [3, 4, 5, 6, 7].into_iter().enumerate() {
        let (mut h, mut shell, mut game) = with_goblin(&format!("fight_beside{n}"), east);
        let at = body_of(&game, 0);
        click(&mut h, &mut shell, &mut game, at);
        for _ in 0..2400 {
            h.advance(0.05);
            turn(&mut h, &mut shell, &mut game);
            if !game.sim.monsters[0].alive() {
                break;
            }
            assert_ne!(game.sim.player.tile, game.sim.monsters[0].tile, "from {east} tiles off");
        }
        assert!(!game.sim.monsters[0].alive());
    }
}

#[test]
fn the_monster_gone_for_is_ringed_on_the_ground_under_it_while_it_is_the_target() {
    let (mut h, mut shell, mut game) = with_goblin("fight_ring", 6);
    let ring = game.look.target_ring.expect("a ring sprite");
    assert!(drawn(&game, ring).is_empty(), "nothing is ringed before a click");
    let at = body_of(&game, 0);
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.target(), Some(0));
    for _ in 0..12 {
        // It goes with the goblin as the goblin moves.
        let rings = drawn(&game, ring);
        let feet = game.monster_grounds()[0];
        assert_eq!(rings.len(), 1);
        assert!((rings[0].ground - feet).length() < 0.001 && rings[0].ground.y < feet.y, "under its feet, and drawn before it");
        pass(&mut h, &mut shell, &mut game, 0.1);
    }
    // Walking off lets it go.
    let (x, y) = game.sim.player.tile;
    let away = screen_of(&game, (x - 2, y));
    click(&mut h, &mut shell, &mut game, away);
    assert!(drawn(&game, ring).is_empty());
    // Fighting back unasked, the one fought is ringed too; dead, it is not.
    until(&mut h, &mut shell, &mut game, 60.0, |game| matches!(game.sim.player.activity(), Some(sim::Activity::Fighting { .. })));
    assert_eq!(drawn(&game, ring).len(), 1);
    until(&mut h, &mut shell, &mut game, 120.0, |game| !game.sim.monsters[0].alive());
    assert!(drawn(&game, ring).is_empty());
}

/// A player with seven ore, one health and no hope, beside a goblin:
/// the game just after it killed them.
pub(super) fn killed(name: &str) -> (Harness, Shell<Game>, Game, (i32, i32)) {
    let (mut h, mut shell, mut game) = with_goblin(name, 2);
    game.sim.inventory.add(Item::CopperOre, 7);
    game.sim.skills.set_xp(Skill::Vitality, 0);
    for _ in 0..2400 {
        game.sim.player.health = game.sim.player.health.min(1);
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        if let Some(fell) = game.sim.grave.as_ref().map(|grave| grave.tile) {
            return (h, shell, game, fell);
        }
    }
    panic!("the goblin never killed them");
}

#[test]
fn dying_says_so_wakes_the_player_at_the_start_and_leaves_a_gravestone() {
    let (_, _, game, fell) = killed("fight_die");
    assert!(game.hud.said().contains(&"You died"));
    assert_eq!(game.sim.player.health, game.sim.max_health());
    assert!(game.sim.inventory.is_empty());
    assert_ne!(game.sim.player.tile, fell, "they woke somewhere else");
    assert_eq!(game.camera.center, game.player_ground(), "and the view is on them at once");
    let stones = drawn(&game, game.look.gravestone.expect("a gravestone sprite"));
    assert_eq!(stones.len(), 1);
    assert_eq!(stones[0].ground, Vec2::new(fell.0 as f64 + 0.5, fell.1 as f64 + 0.5));
    // The grave is in the save already.
    let saved = game.saves.read(0).expect("a save").expect("that reads");
    assert_eq!(saved.grave.map(|g| (g.tile, g.items.count(Item::CopperOre))), Some((fell, 7)));
}

#[test]
fn clicking_the_gravestone_walks_back_and_takes_everything() {
    let (mut h, mut shell, mut game, fell) = killed("fight_grave");
    // No goblin to die to again on the way.
    game.sim.monsters.clear();
    pass(&mut h, &mut shell, &mut game, 1.0);
    let at = screen_of(&game, fell);
    if !game.view.contains(at) {
        // Too far to click from where they woke: walk most of the way.
        game.sim.walk_to((fell.0 - 2, fell.1));
        until(&mut h, &mut shell, &mut game, 120.0, |game| !game.sim.player.busy());
        pass(&mut h, &mut shell, &mut game, 3.0);
    }
    let stone = screen_of(&game, fell);
    click(&mut h, &mut shell, &mut game, stone);
    until(&mut h, &mut shell, &mut game, 60.0, |game| game.sim.grave.is_none());
    assert!(game.sim.grave.is_none());
    assert_eq!(game.sim.inventory.count(Item::CopperOre), 7);
    assert!(game.hud.said().contains(&"You have your things back"));
    assert!(drawn(&game, game.look.gravestone.unwrap()).is_empty());
    assert!(game.saves.read(0).unwrap().unwrap().grave.is_none(), "saved at once");
}

#[test]
fn the_spawn_tool_makes_a_goblin_live_there_once_the_editor_closes() {
    let (mut h, mut shell, mut game) = playing("fight_spawn_tool");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    game.editor.tool = Tool::Spawn;
    let (x, y) = game.sim.player.tile;
    let home = (x + 2, y + 1);
    let at = screen_of(&game, home);
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.world.spawns(), vec![(MonsterKind::Goblin, home.0, home.1)]);
    // Editing, a ghost of it stands on its home; no live goblin yet.
    let (idle, _) = game.look.goblin.sprite(Facing::Down, Pose::Idle).unwrap();
    let ghosts = drawn(&game, idle);
    assert_eq!(ghosts.len(), 1);
    assert!(ghosts[0].tint[3] < 1.0 && ghosts[0].ground == Vec2::new(home.0 as f64 + 0.5, home.1 as f64 + 0.5));
    assert!(game.sim.monsters.is_empty());

    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    assert_eq!(game.sim.monsters.len(), 1);
    assert_eq!(game.sim.monsters[0].home, home);
    // And it is saved with the map like everything else.
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('s'), Modifiers::CTRL);
    assert_eq!(World::load(&game.map).unwrap().spawns(), vec![(MonsterKind::Goblin, home.0, home.1)]);
}

#[test]
fn what_a_fight_earns_reaches_the_save_by_itself() {
    let (mut h, mut shell, mut game) = with_goblin("fight_saves", 3);
    game.autosave();
    until(&mut h, &mut shell, &mut game, 60.0, |game| !game.sim.monsters[0].alive());
    let saved = game.saves.read(0).unwrap().unwrap();
    assert!(saved.skills.xp(Skill::Melee) > 0, "a level gained in the fight reached the save");
}
