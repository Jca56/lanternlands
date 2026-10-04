//! The game being played, driven with no window: clicks walk the
//! player, time passes a frame at a time.

use super::*;

/// Let `seconds` of the game pass, a frame at a time.
fn pass(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, seconds: f64) {
    let frames = (seconds / 0.05).ceil() as usize;
    for _ in 0..frames {
        h.advance(0.05);
        turn(h, shell, game);
    }
}

/// The game with the player stood on a tile of open grass, the view on them.
fn playing(name: &str) -> (Harness, Shell<Game>, Game) {
    let (mut h, mut shell, mut game) = started(name);
    let open = |x: i32, y: i32| (-3..=3).all(|dy| (-3..=3).all(|dx| game.sim.world.walkable(x + dx, y + dy)));
    let (x, y) = (4..28).flat_map(|y| (4..44).map(move |x| (x, y))).find(|&(x, y)| open(x, y)).expect("open ground on the test map");
    game.sim.player.tile = (x, y);
    game.sim.settle_player();
    game.camera.center = Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game)
}

fn screen_of(game: &Game, tile: (i32, i32)) -> Vec2 {
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
    let again = Game::open(&root.join("assets"), game.map.clone());
    assert_eq!(again.sim.player.tile, tile);

    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    assert_eq!(game.extras().len(), 1);
}
