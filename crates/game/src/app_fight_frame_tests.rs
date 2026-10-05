//! Whole frames of a fight, a grave and the editor's goblins, drawn
//! with no window. Set `LANTERNLANDS_RENDER_OUT` to a folder to keep them.

use super::fights::{body_of, killed, until, with_goblin};
use super::frame::{GRASS, body, count, snap};
use super::play::{pass, playing, screen_of};
use super::*;

const GOBLIN_SKIN: [u8; 3] = [0x7b, 0xa0, 0x47];
const HEALTH_LEFT: [u8; 3] = [0x8f, 0xce, 0x5c];
const HEALTH_LOST: [u8; 3] = [0x7a, 0x2e, 0x2a];
const STONE: [u8; 3] = [0x9a, 0x9c, 0x9b];
const HURT: [u8; 3] = [0xff, 0x4a, 0x3d];
const RING: [u8; 3] = [0xe2, 0x48, 0x3c];
/// The walk ring's gold, as it comes out over grass.
const GOLD: [u8; 3] = [0xba, 0x9c, 0x4a];

/// The square of screen a thing standing on `ground` is drawn in.
fn around(game: &Game, ground: Vec2, tiles: f64) -> Rect {
    let px = game.camera.tile_px(game.scale);
    let feet = game.camera.to_screen(ground, game.view, game.scale);
    Rect::from_center_size(feet - Vec2::new(0.0, px * 0.5), Vec2::new(px * tiles, px * tiles))
}

#[test]
fn a_fight_shows_the_goblin_its_bar_and_the_players_health() {
    let (mut h, mut shell, mut game) = with_goblin("frame_fight", 3);
    game.sim.skills.set_xp(Skill::Vitality, 0);
    game.sim.player.health = game.sim.max_health();
    until(&mut h, &mut shell, &mut game, 30.0, |game| matches!(game.sim.player.activity(), Some(sim::Activity::Fighting { .. })));
    // Play on to the moment one of the goblin's blows lands, with the
    // goblin hurt and still standing.
    let mut before = game.sim.player.health;
    for _ in 0..2000 {
        game.sim.monsters[0].health = game.sim.monsters[0].health.max(5);
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        if game.sim.player.health < before && game.sim.monsters[0].health < 12 {
            break;
        }
        before = game.sim.player.health;
    }
    assert!(game.sim.monsters[0].alive() && game.sim.monsters[0].health < 12 && game.sim.player.health < before);
    let Some(image) = snap("frame-fight", &mut h, &mut shell, &mut game) else { return };

    // What it cost them is written in red over their own head, and
    // nowhere near the goblin's.
    let px = game.camera.tile_px(game.scale);
    let head = |ground: Vec2| Rect::from_center_size(game.camera.to_screen(ground - Vec2::new(0.0, 2.0), game.view, game.scale), Vec2::new(px * 0.9, px * 0.7));
    let (mine, its) = (head(game.player_ground()), head(game.monster_grounds()[0]));
    assert!(count(&image, mine, HURT, 6) > 40, "red over the player: {}", count(&image, mine, HURT, 6));
    assert_eq!(count(&image, its, HURT, 6), 0, "and none over the goblin");
    // Under it, over their head, their own bar: some left, some lost.
    let own = Rect::from_center_size(game.camera.to_screen(game.player_ground() - Vec2::new(0.0, 1.95), game.view, game.scale), Vec2::new(px * 1.1, px * 0.3));
    assert!(count(&image, own, HEALTH_LEFT, 2) > 300 && count(&image, own, HEALTH_LOST, 2) > 50, "the player's bar: left {} lost {}", count(&image, own, HEALTH_LEFT, 2), count(&image, own, HEALTH_LOST, 2));
    // The goblin they are fighting is ringed in red on the ground.
    let feet = Rect::from_center_size(game.camera.to_screen(game.monster_grounds()[0], game.view, game.scale), Vec2::new(px, px));
    assert!(count(&image, feet, RING, 3) > 300, "the ring under it: {}", count(&image, feet, RING, 3));

    let goblin = around(&game, game.monster_grounds()[0], 1.4);
    assert!(count(&image, goblin, GOBLIN_SKIN, 6) > 400, "the goblin is drawn where it stands");
    // Its bar, over its head: some health left, some lost.
    let over = Rect::from_center_size(body_of(&game, 0) - Vec2::new(0.0, px * 0.75), Vec2::new(px, px * 0.3));
    assert!(count(&image, over, HEALTH_LEFT, 2) > 50 && count(&image, over, HEALTH_LOST, 2) > 50, "left {} lost {}", count(&image, over, HEALTH_LEFT, 2), count(&image, over, HEALTH_LOST, 2));
    // The player's own bar, top-left: green from the left, red at the right.
    let corner = Rect::new(game.view.min, game.view.min + Vec2::new(400.0, 100.0));
    assert!(count(&image, corner, HEALTH_LEFT, 2) > 2000, "health left");
    assert!(count(&image, corner, HEALTH_LOST, 2) > 200, "health lost");
    assert!(count(&image, corner, [255, 255, 255], 30) > 60, "the numbers on it");
}

#[test]
fn a_goblin_at_peace_has_no_bar_and_nothing_floats() {
    let (mut h, mut shell, mut game) = with_goblin("frame_peace", 7);
    assert!(!game.sim.monsters[0].angry);
    let Some(image) = snap("frame-goblin-idle", &mut h, &mut shell, &mut game) else { return };
    let at = game.monster_grounds()[0];
    assert!(count(&image, around(&game, at, 1.4), GOBLIN_SKIN, 6) > 400, "the goblin stands there");
    assert_eq!(count(&image, around(&game, at, 2.4), HEALTH_LOST, 2), 0);
    // Whole and at peace, the player has no bar over their head either.
    let px = game.camera.tile_px(game.scale);
    let own = Rect::from_center_size(game.camera.to_screen(game.player_ground() - Vec2::new(0.0, 1.95), game.view, game.scale), Vec2::new(px * 1.1, px * 0.3));
    assert_eq!(count(&image, own, HEALTH_LEFT, 2), 0);
    // Full health: the corner bar is all green.
    let corner = Rect::new(game.view.min, game.view.min + Vec2::new(400.0, 100.0));
    assert_eq!(count(&image, corner, HEALTH_LOST, 2), 0);
}

#[test]
fn after_dying_the_gravestone_stands_where_they_fell() {
    let (mut h, mut shell, mut game, fell) = killed("frame_grave");
    game.sim.monsters.clear();
    // Stand beside it so both are in one picture.
    game.sim.player.tile = (fell.0 - 2, fell.1);
    game.sim.player.from = game.sim.player.tile;
    game.camera.center = game.player_ground();
    turn(&mut h, &mut shell, &mut game);
    let Some(image) = snap("frame-grave", &mut h, &mut shell, &mut game) else { return };
    let stone = around(&game, Vec2::new(fell.0 as f64 + 0.5, fell.1 as f64 + 0.5), 1.0);
    let bare = around(&game, Vec2::new(fell.0 as f64 + 0.5, fell.1 as f64 + 3.5), 1.0);
    assert!(count(&image, stone, GRASS, 1) + 2000 < count(&image, bare, GRASS, 1), "something stands on the grass there: {} against {}", count(&image, stone, GRASS, 1), count(&image, bare, GRASS, 1));
    assert!(count(&image, stone, STONE, 8) > 300, "and it is stone grey: {}", count(&image, stone, STONE, 8));
    // The message is still at the top, and the blow that killed them
    // does not float over where they woke.
    assert!(game.hud.said().contains(&"You died"));
    assert!(game.hud.floating().is_empty(), "{:?}", game.hud.floating());
    pass(&mut h, &mut shell, &mut game, 0.1);
}

#[test]
fn the_editor_lists_monsters_and_shows_a_ghost_where_one_lives() {
    let (mut h, mut shell, mut game) = playing("frame_spawn");
    let (x, y) = game.sim.player.tile;
    assert!(game.sim.world.set_spawn(x + 2, y, Some(sim::MonsterKind::Goblin)));
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    panel_tab(&mut h, &mut shell, &mut game, 2);
    let Some(image) = snap("frame-editor-monsters", &mut h, &mut shell, &mut game) else { return };
    let row = h.rect_of(body(&shell, View::Editor).with("map editor").with(panel::MONSTERS).with("Goblin")).expect("a Goblin row");
    assert!(count(&image, row, GOBLIN_SKIN, 12) > 40, "its picture is in the row: {}", count(&image, row, GOBLIN_SKIN, 12));
    // The ghost: goblin-coloured, but thinned by the grass behind it.
    let ghost = around(&game, Vec2::new((x + 2) as f64 + 0.5, y as f64 + 0.5), 1.4);
    assert!(count(&image, ghost, GRASS, 1) < (ghost.area() * 0.9) as usize, "something is drawn on its home");
    assert_eq!(count(&image, ghost, GOBLIN_SKIN, 2), 0, "and it is a ghost, not the full colour");
}

/// The square of screen a tile takes.
fn tile_rect(game: &Game, tile: (i32, i32)) -> Rect {
    let px = game.camera.tile_px(game.scale);
    Rect::from_center_size(screen_of(game, tile), Vec2::new(px, px))
}

#[test]
fn a_click_on_the_ground_rings_the_spot_and_a_click_on_a_goblin_rings_the_goblin_in_red() {
    let (mut h, mut shell, mut game) = with_goblin("frame_rings", 6);
    let (x, y) = game.sim.player.tile;
    // The grass as it is, to tell a ring from.
    let Some(plain) = snap("frame-ring-none", &mut h, &mut shell, &mut game) else { return };
    let window = Rect::new(game.view.min, game.view.max);
    assert_eq!(count(&plain, window, RING, 3), 0, "no red ring with nothing clicked");

    // Walking: the spot walked to is ringed, in the walk's own colour.
    let spot = (x - 3, y);
    let at = screen_of(&game, spot);
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.player.goal(), Some(spot));
    let Some(walk) = snap("frame-ring-walk", &mut h, &mut shell, &mut game) else { return };
    let ringed = tile_rect(&game, spot);
    assert_eq!(count(&walk, window, RING, 3), 0, "walking rings nothing in red");
    assert!(count(&walk, ringed, GOLD, 16) > 150, "the spot's ring: {}", count(&walk, ringed, GOLD, 16));

    // Going for the goblin: it is ringed in red, and the ground they
    // will stand on is not ringed at all.
    game.sim.stop();
    game.sim.player.tile = (x, y);
    game.sim.settle_player();
    game.camera.center = game.player_ground();
    pass(&mut h, &mut shell, &mut game, 0.1);
    let at = body_of(&game, 0);
    click(&mut h, &mut shell, &mut game, at);
    let goal = game.sim.player.goal().expect("on the way to it");
    let Some(attack) = snap("frame-ring-attack", &mut h, &mut shell, &mut game) else { return };
    assert_eq!(count(&attack, tile_rect(&game, goal), GOLD, 16), 0, "no ring on the ground they are walking to");
    let px = game.camera.tile_px(game.scale);
    let feet = Rect::from_center_size(game.camera.to_screen(game.monster_grounds()[0], game.view, game.scale), Vec2::new(px, px));
    assert!(count(&attack, feet, RING, 3) > 300, "the goblin's ring: {}", count(&attack, feet, RING, 3));
}
