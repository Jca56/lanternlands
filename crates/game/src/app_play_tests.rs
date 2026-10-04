//! The game being played, driven with no window: clicks walk the
//! player, time passes a frame at a time.

use super::*;

/// Let `seconds` of the game pass, a frame at a time.
pub(super) fn pass(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, seconds: f64) {
    let frames = (seconds / 0.05).ceil() as usize;
    for _ in 0..frames {
        h.advance(0.05);
        turn(h, shell, game);
    }
}

/// The game with the player stood on a tile of open grass, the view on them.
pub(super) fn playing(name: &str) -> (Harness, Shell<Game>, Game) {
    let (mut h, mut shell, mut game) = started(name);
    let open = |x: i32, y: i32| (-3..=3).all(|dy| (-3..=3).all(|dx| game.sim.world.walkable(x + dx, y + dy)));
    let (x, y) = (4..28).flat_map(|y| (4..44).map(move |x| (x, y))).find(|&(x, y)| open(x, y)).expect("open ground on the test map");
    game.sim.player.tile = (x, y);
    game.sim.settle_player();
    game.camera.center = Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game)
}

pub(super) fn screen_of(game: &Game, tile: (i32, i32)) -> Vec2 {
    game.camera.to_screen(Vec2::new(tile.0 as f64 + 0.5, tile.1 as f64 + 0.5), game.view, game.scale)
}

#[test]
fn the_player_starts_somewhere_they_can_stand_with_the_view_on_them() {
    let (_, _, game) = started("start_place");
    let (x, y) = game.sim.player.tile;
    assert!(game.sim.world.walkable(x, y));
    assert_eq!(game.camera.center, Vec2::new(x as f64 + 0.5, y as f64 + 0.5));
}

#[test]
fn a_click_walks_the_player_there_one_tile_a_tick() {
    let (mut h, mut shell, mut game) = playing("walk");
    let (x, y) = game.sim.player.tile;
    let target = (x + 3, y + 1);
    let at = screen_of(&game, target);
    h.move_to(at);
    turn(&mut h, &mut shell, &mut game);
    h.press();
    turn(&mut h, &mut shell, &mut game);
    h.release();
    assert_eq!(game.sim.player.goal(), Some(target));
    assert!(!game.sim.world.is_dirty(), "walking is not editing");

    // Three steps away: not there after two ticks, there after three.
    pass(&mut h, &mut shell, &mut game, 1.5 * TICK_SECONDS);
    assert_ne!(game.sim.player.tile, target);
    pass(&mut h, &mut shell, &mut game, 2.0 * TICK_SECONDS);
    assert_eq!(game.sim.player.tile, target);
    assert_eq!(game.sim.player.steps, 3);
    // And once the last step is finished they stand still, facing the way they came.
    pass(&mut h, &mut shell, &mut game, 2.0 * TICK_SECONDS);
    assert!(!game.sim.player.moved() && game.sim.player.goal().is_none());
    assert_eq!(game.facing, Facing::Right);
    assert_eq!(game.player_ground(), Vec2::new(target.0 as f64 + 0.5, target.1 as f64 + 0.5));
}

#[test]
fn the_first_step_starts_at_once_and_the_player_slides_between_tiles() {
    let (mut h, mut shell, mut game) = playing("slide");
    let (x, y) = game.sim.player.tile;
    h.move_to(screen_of(&game, (x + 2, y)));
    turn(&mut h, &mut shell, &mut game);
    h.press();
    h.advance(0.05);
    turn(&mut h, &mut shell, &mut game);
    h.release();
    // No waiting for a tick to come round: they are already on their way.
    assert_eq!(game.sim.player.tile, (x + 1, y));
    pass(&mut h, &mut shell, &mut game, 0.15);
    let ground = game.player_ground();
    assert!(ground.x > x as f64 + 0.5 && ground.x < x as f64 + 1.5 && ground.y == y as f64 + 0.5, "partway between the tiles: {ground:?}");
}

#[test]
fn the_player_is_drawn_striding_while_they_walk_and_idle_when_they_stop() {
    let (mut h, mut shell, mut game) = playing("poses");
    let walk = game.look.player;
    let player = |game: &Game| *game.extras().last().expect("the player is drawn");
    assert_eq!(player(&game).sprite, walk.sprite(Facing::Down, Pose::Idle).unwrap().0);

    // Walk left: the side view, mirrored.
    let (x, y) = game.sim.player.tile;
    h.move_to(screen_of(&game, (x - 2, y)));
    turn(&mut h, &mut shell, &mut game);
    h.press();
    h.advance(0.05);
    turn(&mut h, &mut shell, &mut game);
    h.release();
    let striding = player(&game);
    let strides = [Pose::StepA, Pose::StepB].map(|pose| walk.sprite(Facing::Left, pose).unwrap().0);
    assert!(strides.contains(&striding.sprite) && striding.flip, "a mirrored stride at the start of a step");
    pass(&mut h, &mut shell, &mut game, 0.25);
    assert_eq!(player(&game).sprite, walk.sprite(Facing::Left, Pose::Pass).unwrap().0, "feet together in its second half");

    pass(&mut h, &mut shell, &mut game, 3.0 * TICK_SECONDS);
    let stopped = player(&game);
    assert_eq!((stopped.sprite, stopped.flip), walk.sprite(Facing::Left, Pose::Idle).unwrap());
}

#[test]
fn the_view_glides_after_the_player_and_ends_on_them() {
    let (mut h, mut shell, mut game) = playing("follow");
    let (x, y) = game.sim.player.tile;
    h.move_to(screen_of(&game, (x + 3, y)));
    turn(&mut h, &mut shell, &mut game);
    h.press();
    turn(&mut h, &mut shell, &mut game);
    h.release();
    pass(&mut h, &mut shell, &mut game, 2.0 * TICK_SECONDS);
    let behind = game.player_ground().x - game.camera.center.x;
    assert!(behind > 0.0 && behind < 1.5, "the view trails the player a little while they walk: {behind}");
    pass(&mut h, &mut shell, &mut game, 4.0);
    assert_eq!(game.camera.center, Vec2::new(x as f64 + 3.5, y as f64 + 0.5));
}

#[test]
fn nothing_asks_for_frames_once_the_player_and_the_view_are_at_rest() {
    let (mut h, mut shell, mut game) = playing("rest");
    pass(&mut h, &mut shell, &mut game, 1.0);
    h.advance(0.05);
    let out = turn(&mut h, &mut shell, &mut game);
    assert_eq!(out.wake_after, None, "an idle game draws no frames");
}

#[test]
fn the_game_stands_still_while_the_editor_is_open() {
    let (mut h, mut shell, mut game) = playing("paused");
    let (x, y) = game.sim.player.tile;
    h.move_to(screen_of(&game, (x + 3, y)));
    turn(&mut h, &mut shell, &mut game);
    h.press();
    h.advance(0.05);
    turn(&mut h, &mut shell, &mut game);
    h.release();
    assert_eq!(game.sim.player.tile, (x + 1, y));

    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    let ticks = game.sim.ticks;
    pass(&mut h, &mut shell, &mut game, 2.0);
    assert_eq!((game.sim.ticks, game.sim.player.tile), (ticks, (x + 1, y)));

    // Closing it, they stay where they were and are going nowhere.
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    pass(&mut h, &mut shell, &mut game, 2.0);
    assert_eq!(game.sim.player.tile, (x + 1, y));
    assert_eq!(game.sim.player.goal(), None);
}

#[test]
fn a_player_whose_tile_was_edited_away_is_moved_off_it_when_the_editor_closes() {
    let (mut h, mut shell, mut game) = playing("edited_away");
    let (x, y) = game.sim.player.tile;
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    game.sim.world.set_terrain(x, y, Terrain::Water);
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    let (nx, ny) = game.sim.player.tile;
    assert_ne!((nx, ny), (x, y));
    assert!(game.sim.world.walkable(nx, ny));
}

#[test]
fn the_start_tool_sets_the_start_and_shows_a_ghost_there_only_while_editing() {
    let (mut h, mut shell, mut game) = playing("start_tool");
    assert_eq!(game.extras().len(), 1, "playing, only the player is extra");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    let row = panel_row(&h, &shell, "Start");
    click(&mut h, &mut shell, &mut game, row);
    assert_eq!(game.editor.tool, Tool::Start);

    let (x, y) = game.sim.player.tile;
    let tile = (x + 2, y + 2);
    let at = screen_of(&game, tile);
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.world.marker(Marker::PlayerStart), Some(tile));
    let ghost = game.extras()[0];
    assert_eq!(ghost.ground, Vec2::new(tile.0 as f64 + 0.5, tile.1 as f64 + 0.5));
    assert!(ghost.tint[3] < 1.0, "see-through");

    // Saved with the map, it is where the next game starts.
    key(&mut h, &mut shell, &mut game, Key::Char('s'), Modifiers::CTRL);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut again = Game::open(&root.join("assets"), game.map.clone(), root.join("target/tmp/game/start_tool/other_saves"));
    again.enter(0, Character { name: "Newcomer".to_owned(), played: 0 }, None);
    assert_eq!(again.sim.player.tile, tile);

    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    assert_eq!(game.extras().len(), 1);
}

// ---- mining -----------------------------------------------------------------

/// The game with the player on open grass and a rock two tiles east.
pub(super) fn at_a_rock(name: &str) -> (Harness, Shell<Game>, Game, (i32, i32)) {
    let (mut h, mut shell, mut game) = playing(name);
    let (x, y) = game.sim.player.tile;
    let rock = (x + 2, y);
    assert!(game.sim.world.place_prop(PropKind::Rock, rock.0, rock.1));
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game, rock)
}

/// Press the left button on a tile of the world.
pub(super) fn press_tile(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, tile: (i32, i32)) {
    let at = screen_of(game, tile);
    h.move_to(at);
    turn(h, shell, game);
    h.press();
    turn(h, shell, game);
    h.release();
    turn(h, shell, game);
}

#[test]
fn clicking_a_rock_mines_it_out_into_the_bag() {
    let (mut h, mut shell, mut game, rock) = at_a_rock("mine");
    press_tile(&mut h, &mut shell, &mut game, rock);
    let mut floated = Vec::new();
    for _ in 0..600 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        for line in game.hud.floating() {
            if !floated.contains(&line.to_owned()) {
                floated.push(line.to_owned());
            }
        }
        if game.sim.is_spent(rock) {
            break;
        }
    }
    assert!(game.sim.is_spent(rock), "the rock was mined out");
    assert_eq!(game.sim.inventory.count(Item::CopperOre), 3);
    assert_eq!(game.sim.skills.level(Skill::Mining), 2);
    assert_eq!(floated, vec!["+1 Copper ore"], "what was got floated up from the player, and no experience with it");
    assert_eq!(game.sim.player.tile, (rock.0 - 1, rock.1), "they stood beside it");
    assert_eq!(game.facing, Facing::Right, "facing it");
    assert_eq!(game.sim.world.prop_at(rock.0, rock.1).map(|p| p.kind), Some(PropKind::Rock), "the rock is still a rock on the map");
}

/// The pose the player is drawn in.
pub(super) fn pose_of(game: &Game) -> Option<Pose> {
    let drawn = game.extras();
    Pose::ALL.into_iter().find(|&pose| game.look.player.sprite(game.facing, pose).is_some_and(|(sprite, flip)| drawn.iter().any(|p| p.sprite == sprite && p.flip == flip)))
}

/// How many separate stretches of `pose` there are in a row of frames.
pub(super) fn runs(poses: &[Pose], pose: Pose) -> usize {
    poses.iter().enumerate().filter(|&(i, p)| *p == pose && (i == 0 || poses[i - 1] != pose)).count()
}

#[test]
fn mining_is_one_swing_for_each_roll_and_standing_ready_between() {
    let (mut h, mut shell, mut game, rock) = at_a_rock("work_pose");
    press_tile(&mut h, &mut shell, &mut game, rock);
    let (mut poses, mut rolls, mut swung) = (Vec::new(), 0, false);
    for _ in 0..2400 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        rolls += (game.sim.player.swung && !swung) as usize;
        swung = game.sim.player.swung;
        poses.push(pose_of(&game).expect("the player is drawn"));
        if !game.sim.player.busy() && !game.sim.player.swung {
            break;
        }
    }
    assert!(rolls >= 3, "the rock was mined out: {rolls} swings");
    assert_eq!(runs(&poses, Pose::WorkB), rolls, "one strike for each swing, the last one too");
    assert_eq!(runs(&poses, Pose::WorkA), rolls, "and one drawing back before each");
    for (i, pose) in poses.iter().enumerate().skip(1) {
        if *pose == Pose::WorkB && poses[i - 1] != Pose::WorkB {
            assert_eq!(poses[i - 1], Pose::WorkA, "a strike follows its drawing back");
        }
    }
    let at_work = poses.iter().filter(|p| matches!(p, Pose::WorkA | Pose::WorkB)).count();
    let ready = poses.iter().filter(|p| **p == Pose::Idle).count();
    assert!(ready > at_work, "more of the time is spent standing ready ({ready} frames) than swinging ({at_work})");
    assert_eq!(poses.last(), Some(&Pose::Idle));
    assert_eq!(game.facing, Facing::Right);
}

#[test]
fn a_spent_rock_comes_back_in_real_time_with_the_game_drawing_only_on_ticks() {
    let (mut h, mut shell, mut game, rock) = at_a_rock("respawn");
    press_tile(&mut h, &mut shell, &mut game, rock);
    for _ in 0..600 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        if game.sim.is_spent(rock) {
            break;
        }
    }
    // Let the floating text and the level-up message fade and the view settle.
    pass(&mut h, &mut shell, &mut game, 5.0);
    let spent_tick = game.sim.ticks;
    // Idle with a rock on its way back: a frame is asked for at the next
    // tick, not at once.
    h.advance(0.1);
    let out = turn(&mut h, &mut shell, &mut game);
    let wake = out.wake_after.expect("a wake for the next tick");
    assert!(wake > 0.0 && wake <= TICK_SECONDS, "wake in {wake}");
    // Frames a whole tick apart still count every tick.
    for _ in 0..50 {
        h.advance(TICK_SECONDS);
        turn(&mut h, &mut shell, &mut game);
    }
    assert!(game.sim.ticks - spent_tick >= 49, "{} ticks in 50 frames a tick apart", game.sim.ticks - spent_tick);
    pass(&mut h, &mut shell, &mut game, 30.0);
    assert_eq!(game.sim.node_left(rock), Some(3), "whole again");
    // And then nothing asks for frames at all.
    h.advance(0.05);
    assert_eq!(turn(&mut h, &mut shell, &mut game).wake_after, None);
}

#[test]
fn the_bag_and_the_skills_open_with_their_keys_and_their_buttons() {
    let (mut h, mut shell, mut game) = playing("panels");
    assert!(!game.hud.bag_open && !game.hud.skills_open);
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    assert!(game.hud.bag_open);
    key(&mut h, &mut shell, &mut game, Key::Char('k'), Modifiers::NONE);
    assert!(game.hud.skills_open);
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    assert!(!game.hud.bag_open && game.hud.skills_open);

    // The buttons sit in the window's bottom-right corner: Skills, then Bag.
    let body = WidgetId::ROOT.with_u64(shell.screen.target(View::World).unwrap() as u64).with("body");
    let bag = h.rect_of(body.with("Bag")).expect("a Bag button");
    let skills = h.rect_of(body.with("Skills")).expect("a Skills button");
    assert!(bag.max.x <= WINDOW.x && bag.max.y <= WINDOW.y && bag.min.x > WINDOW.x - 400.0 && skills.max.x <= bag.min.x);
    assert!(bag.height() >= 55.0 && bag.width() >= 130.0, "a big target: {bag:?}");
    click(&mut h, &mut shell, &mut game, bag.center());
    assert!(game.hud.bag_open);
    click(&mut h, &mut shell, &mut game, skills.center());
    assert!(!game.hud.skills_open);
}

#[test]
fn a_click_on_a_panel_or_a_button_does_not_walk_the_player() {
    let (mut h, mut shell, mut game) = playing("shield");
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    let body = WidgetId::ROOT.with_u64(shell.screen.target(View::World).unwrap() as u64).with("body");
    let slot = h.rect_of(body.with("slot").with_index(7)).expect("the bag's slots are laid out");
    assert!(slot.width() >= 75.0 && slot.height() >= 75.0, "a big slot: {slot:?}");
    click(&mut h, &mut shell, &mut game, slot.center());
    assert_eq!(game.sim.player.goal(), None);
    assert!(!game.sim.player.busy());
}

#[test]
fn the_bag_has_thirty_slots_five_across_and_six_down() {
    let (mut h, mut shell, mut game) = playing("slots");
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    let body = WidgetId::ROOT.with_u64(shell.screen.target(View::World).unwrap() as u64).with("body");
    let slots: Vec<Rect> = (0..30).map(|i| h.rect_of(body.with("slot").with_index(i)).expect("a slot")).collect();
    assert!(h.rect_of(body.with("slot").with_index(30)).is_none());
    let first_row = slots.iter().filter(|s| s.min.y == slots[0].min.y).count();
    let first_column = slots.iter().filter(|s| s.min.x == slots[0].min.x).count();
    assert_eq!((first_row, first_column), (5, 6), "taller than wide");
    // Slots run left to right, then down: the sixth starts the second row.
    assert!(slots[5].min.x == slots[0].min.x && slots[5].min.y > slots[0].min.y);
    // No two overlap, and all are inside the window.
    for (i, a) in slots.iter().enumerate() {
        assert!(a.min.x >= 0.0 && a.max.x <= WINDOW.x && a.min.y >= 0.0 && a.max.y <= WINDOW.y);
        assert!(slots[i + 1..].iter().all(|b| !a.intersects(b)), "slot {i} overlaps another");
    }
}

#[test]
fn the_interface_is_put_away_while_editing() {
    let (mut h, mut shell, mut game) = playing("hud_editing");
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    let body = WidgetId::ROOT.with_u64(shell.screen.target(View::World).unwrap() as u64).with("body");
    assert!(h.rect_of(body.with("Bag")).is_none() && h.rect_of(body.with("slot").with_index(0)).is_none());
    // The keys that open panels do nothing here.
    key(&mut h, &mut shell, &mut game, Key::Char('k'), Modifiers::NONE);
    assert!(!game.hud.skills_open);
}
