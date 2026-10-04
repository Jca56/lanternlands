//! The whole game driven with no window: a shell, the host, and
//! pretend keys and clicks, the way `lntrn-app` would feed them.

use lntrn_ui::testing::Harness;
use lntrn_ui::{Modifiers, ShellOutput, WidgetId};
use sim::{Character, Item, Marker, Skill, Terrain};

use super::*;
use crate::anim::Pose;
use crate::editor::Tool;

const WINDOW: Vec2 = Vec2::new(1600.0, 1000.0);

/// One turn of the app's loop: rebuild until the shell is settled, then
/// let the host change the layout, and lay out again if it did.
fn turn(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game) -> ShellOutput {
    let out = h.shell_settle(shell, game, 4);
    if game.sync_panel(shell) {
        h.shell_settle(shell, game, 4);
    }
    out
}

/// A fresh game on a copy of the test map and an empty save folder, in
/// scratch folders of their own: on the title screen.
fn opened(name: &str) -> (Harness, Shell<Game>, Game) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scratch = root.join("target/tmp/game").join(name);
    let _ = std::fs::remove_dir_all(&scratch);
    sim::testmap::build().save(&scratch.join("map")).expect("the map is written");
    let mut game = Game::open(&root.join("assets"), scratch.join("map"), scratch.join("saves"));
    let mut shell = Shell::new(View::World);
    shell.title_bar = false;
    let mut h = Harness::new(WINDOW.x, WINDOW.y);
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game)
}

/// The same with a new character entered: playing.
fn started(name: &str) -> (Harness, Shell<Game>, Game) {
    let (mut h, mut shell, mut game) = opened(name);
    game.enter(0, Character { name: "Tester".to_owned(), played: 0 }, None);
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game)
}

fn key(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, key: Key, mods: Modifiers) -> ShellOutput {
    h.key_with(key, mods);
    turn(h, shell, game)
}

/// Click the left button at a point of the window.
fn click(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, at: Vec2) -> ShellOutput {
    h.move_to(at);
    turn(h, shell, game);
    h.press();
    turn(h, shell, game);
    h.release();
    turn(h, shell, game)
}

/// Where a row of the editor's panel is on screen.
fn panel_row(h: &Harness, shell: &Shell<Game>, label: &str) -> Vec2 {
    let area = shell.screen.target(View::Editor).expect("the panel is open");
    let id = WidgetId::ROOT.with_u64(area as u64).with("body").with("map editor").with(label);
    h.rect_of(id).unwrap_or_else(|| panic!("no row called {label} on the panel")).center()
}

/// Where the open dialog's button number `index` (from the left),
/// labelled `label`, is on screen.
fn dialog_button(h: &Harness, index: usize, label: &str) -> Vec2 {
    let id = WidgetId::ROOT.with("popup").with("popup").with_index(index).with(label);
    h.rect_of(id).unwrap_or_else(|| panic!("button {index} of the dialog is not {label}")).center()
}

/// The tile under the middle of the world's area.
fn middle_tile(game: &Game) -> (i32, i32) {
    let at = game.camera.to_world(game.view.center(), game.view, game.scale);
    (at.x.floor() as i32, at.y.floor() as i32)
}

/// Open the editor, pick water, and paint the tile in the middle.
fn paint_the_middle(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game) -> (i32, i32) {
    key(h, shell, game, Key::F(2), Modifiers::NONE);
    assert!(game.editing);
    game.editor.terrain = Terrain::Water;
    let tile = middle_tile(game);
    assert_ne!(game.sim.world.terrain(tile.0, tile.1), Some(Terrain::Water));
    let middle = game.view.center();
    click(h, shell, game, middle);
    assert_eq!(game.sim.world.terrain(tile.0, tile.1), Some(Terrain::Water));
    tile
}

#[test]
fn the_map_loads_clean_and_the_world_has_the_whole_window() {
    let (_, shell, game) = started("loads");
    assert!(!game.sim.world.is_dirty());
    assert_eq!(game.sim.world.chunk_count(), 2);
    assert_eq!(shell.screen.area_count(), 1);
    assert_eq!(game.view, Rect::new(Vec2::ZERO, WINDOW));
    assert!(!game.editing);
}

#[test]
fn f2_docks_the_panel_on_the_left_and_f2_again_takes_it_away() {
    let (mut h, mut shell, mut game) = started("panel");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    assert!(game.editing);
    assert_eq!(shell.screen.area_count(), 2);
    let rect_of = |shell: &Shell<Game>, view| shell.screen.layout_of(shell.screen.target(view).expect("on screen")).expect("laid out").rect;
    let (panel, world) = (rect_of(&shell, View::Editor), rect_of(&shell, View::World));
    assert_eq!(panel.min.x, 0.0, "the panel is on the left");
    assert!((panel.width() - PANEL_WIDTH).abs() <= 10.0, "the panel is {} wide", panel.width());
    assert!(world.min.x >= panel.max.x && world.max.x == WINDOW.x, "the world has the rest");
    assert_eq!(game.view, world, "the world is drawn in its own area");

    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    assert!(!game.editing);
    assert_eq!(shell.screen.area_count(), 1);
    assert_eq!(game.view, Rect::new(Vec2::ZERO, WINDOW));
}

#[test]
fn a_click_on_the_world_does_nothing_until_the_editor_is_open() {
    let (mut h, mut shell, mut game) = started("closed");
    game.editor.terrain = Terrain::Water;
    click(&mut h, &mut shell, &mut game, WINDOW * 0.5);
    assert!(!game.sim.world.is_dirty());
    assert!(!game.editor.history.can_undo());
}

#[test]
fn a_click_paints_the_tile_under_the_pointer_and_undo_and_redo_work_from_the_keys() {
    let (mut h, mut shell, mut game) = started("paint");
    let (x, y) = middle_tile(&game);
    let before = game.sim.world.terrain(x, y);
    let (x, y) = paint_the_middle(&mut h, &mut shell, &mut game);
    assert!(game.sim.world.is_dirty());

    key(&mut h, &mut shell, &mut game, Key::Char('z'), Modifiers::CTRL);
    assert_eq!(game.sim.world.terrain(x, y), before);
    key(&mut h, &mut shell, &mut game, Key::Char('Z'), Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(game.sim.world.terrain(x, y), Some(Terrain::Water));
    key(&mut h, &mut shell, &mut game, Key::Char('z'), Modifiers::CTRL);
    key(&mut h, &mut shell, &mut game, Key::Char('y'), Modifiers::CTRL);
    assert_eq!(game.sim.world.terrain(x, y), Some(Terrain::Water));
}

#[test]
fn a_drag_paints_a_line_as_one_step() {
    let (mut h, mut shell, mut game) = started("drag");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    game.editor.terrain = Terrain::Sand;
    let from = game.view.center() + Vec2::new(-250.0, -200.0);
    let (a, b) = (game.camera.to_world(from, game.view, game.scale), game.camera.to_world(from + Vec2::new(500.0, 0.0), game.view, game.scale));
    let sand = |game: &Game| (a.x.floor() as i32..=b.x.floor() as i32).filter(|&x| game.sim.world.terrain(x, a.y.floor() as i32) == Some(Terrain::Sand)).count();
    let already = sand(&game);

    h.move_to(from);
    turn(&mut h, &mut shell, &mut game);
    h.press();
    turn(&mut h, &mut shell, &mut game);
    // One jump of five tiles: the pointer outran the frames.
    h.move_to(from + Vec2::new(500.0, 0.0));
    turn(&mut h, &mut shell, &mut game);
    h.release();
    turn(&mut h, &mut shell, &mut game);
    assert_eq!(sand(&game), 6, "every tile the drag crossed is sand");

    key(&mut h, &mut shell, &mut game, Key::Char('z'), Modifiers::CTRL);
    assert_eq!(sand(&game), already, "one undo takes the whole stroke");
    assert!(!game.editor.history.can_undo());
}

#[test]
fn the_panels_rows_pick_what_the_next_click_does() {
    let (mut h, mut shell, mut game) = started("rows");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    let row = panel_row(&h, &shell, "Rock");
    click(&mut h, &mut shell, &mut game, row);
    assert_eq!((game.editor.tool, game.editor.prop), (Tool::Place, PropKind::Rock));

    // A grass tile with nothing on it, near the middle of the view.
    let (mx, my) = middle_tile(&game);
    let (x, y) = (my - 3..my + 3).flat_map(|y| (mx - 3..mx + 3).map(move |x| (x, y))).find(|&(x, y)| game.sim.world.terrain(x, y) == Some(Terrain::Grass) && game.sim.world.prop_at(x, y).is_none()).expect("bare grass in view");
    let at = game.camera.to_screen(Vec2::new(x as f64 + 0.5, y as f64 + 0.5), game.view, game.scale);
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.world.prop_at(x, y).map(|p| p.kind), Some(PropKind::Rock));
}

#[test]
fn the_move_tool_drags_a_prop_to_another_tile_and_one_undo_puts_it_back() {
    let (mut h, mut shell, mut game) = started("move");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    let row = panel_row(&h, &shell, "Move");
    click(&mut h, &mut shell, &mut game, row);
    assert_eq!(game.editor.tool, Tool::Move);

    // Two bare grass tiles well inside the view: a tree is stood on the
    // first, to be carried to the second.
    let seen = game.camera.visible(game.view.size(), game.scale).shrink(2.0);
    let (x0, y0, x1, y1) = (seen.min.x.ceil() as i32, seen.min.y.ceil() as i32, seen.max.x.floor() as i32, seen.max.y.floor() as i32);
    let bare: Vec<(i32, i32)> = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| game.sim.world.terrain(x, y) == Some(Terrain::Grass) && game.sim.world.prop_at(x, y).is_none()).collect();
    let ((px, py), (tx, ty)) = (bare[0], *bare.last().expect("bare grass in view"));
    assert_ne!((px, py), (tx, ty));
    assert!(game.sim.world.place_prop(PropKind::Tree, px, py));
    let prop = game.sim.world.prop_at(px, py).expect("just placed");
    let on_screen = |game: &Game, x: i32, y: i32| game.camera.to_screen(Vec2::new(x as f64 + 0.5, y as f64 + 0.5), game.view, game.scale);
    let (from, to) = (on_screen(&game, prop.x, prop.y), on_screen(&game, tx, ty));
    let props = game.sim.world.props_in(-100, -100, 200, 200).count();

    h.move_to(from);
    turn(&mut h, &mut shell, &mut game);
    h.press();
    turn(&mut h, &mut shell, &mut game);
    assert_eq!(game.editor.carrying(), Some((prop.x, prop.y)));
    h.move_to(to);
    turn(&mut h, &mut shell, &mut game);
    assert_eq!(game.sim.world.prop_at(tx, ty).map(|p| p.kind), Some(prop.kind), "it follows the pointer before the button is up");
    h.release();
    turn(&mut h, &mut shell, &mut game);

    assert_eq!(game.sim.world.prop_at(prop.x, prop.y), None);
    assert_eq!(game.sim.world.prop_at(tx, ty).map(|p| p.kind), Some(prop.kind));
    assert_eq!(game.sim.world.props_in(-100, -100, 200, 200).count(), props, "nothing was made or lost");
    assert_eq!(game.editor.carrying(), None);

    key(&mut h, &mut shell, &mut game, Key::Char('z'), Modifiers::CTRL);
    assert_eq!(game.sim.world.prop_at(prop.x, prop.y).map(|p| p.kind), Some(prop.kind));
    assert_eq!(game.sim.world.prop_at(tx, ty), None);
    assert!(!game.editor.history.can_undo());
}

#[test]
fn ctrl_s_saves_and_the_edit_is_there_when_the_map_is_loaded_again() {
    let (mut h, mut shell, mut game) = started("save");
    let (x, y) = paint_the_middle(&mut h, &mut shell, &mut game);
    key(&mut h, &mut shell, &mut game, Key::Char('s'), Modifiers::CTRL);
    assert!(!game.sim.world.is_dirty());
    let again = World::load(&game.map).expect("loads");
    assert_eq!(again.terrain(x, y), Some(Terrain::Water));
}

#[test]
fn quitting_clean_just_quits() {
    let (mut h, mut shell, mut game) = started("quit_clean");
    assert!(key(&mut h, &mut shell, &mut game, Key::Char('q'), Modifiers::CTRL).quit);
}

#[test]
fn quitting_with_unsaved_changes_asks_and_discard_quits_without_saving() {
    let (mut h, mut shell, mut game) = started("quit_discard");
    let (x, y) = paint_the_middle(&mut h, &mut shell, &mut game);
    assert!(!key(&mut h, &mut shell, &mut game, Key::Char('q'), Modifiers::CTRL).quit, "it asks first");
    assert!(shell.popup_open());
    // Escape is "cancel": the game is still running and still unsaved.
    assert!(!key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE).quit);
    assert!(!shell.popup_open() && game.sim.world.is_dirty());

    key(&mut h, &mut shell, &mut game, Key::Char('q'), Modifiers::CTRL);
    let discard = dialog_button(&h, 1, "Discard");
    assert!(click(&mut h, &mut shell, &mut game, discard).quit);
    assert_ne!(World::load(&game.map).expect("loads").terrain(x, y), Some(Terrain::Water), "nothing was saved");
}

#[test]
fn quitting_with_unsaved_changes_and_enter_saves_then_quits() {
    let (mut h, mut shell, mut game) = started("quit_save");
    let (x, y) = paint_the_middle(&mut h, &mut shell, &mut game);
    key(&mut h, &mut shell, &mut game, Key::Char('q'), Modifiers::CTRL);
    assert!(key(&mut h, &mut shell, &mut game, Key::Enter, Modifiers::NONE).quit);
    assert_eq!(World::load(&game.map).expect("loads").terrain(x, y), Some(Terrain::Water));
}

#[test]
fn the_window_systems_close_asks_too() {
    let (mut h, mut shell, mut game) = started("close");
    paint_the_middle(&mut h, &mut shell, &mut game);
    h.event(lntrn_ui::Event::CloseRequested);
    let out = turn(&mut h, &mut shell, &mut game);
    assert!(!out.quit && out.window_command.is_none(), "the window stays");
    assert!(shell.popup_open());
}

#[test]
fn a_map_that_does_not_load_says_so_and_starts_empty() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scratch = root.join("target/tmp/game/broken");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(scratch.join("map")).unwrap();
    std::fs::write(scratch.join("map/0_0.chunk"), b"not a chunk").unwrap();
    let mut game = Game::open(&root.join("assets"), scratch.join("map"), scratch.join("saves"));
    let mut shell = Shell::new(View::World);
    shell.title_bar = false;
    let mut h = Harness::new(WINDOW.x, WINDOW.y);
    turn(&mut h, &mut shell, &mut game);
    assert!(shell.popup_open(), "a dialog says the map did not load");
    assert_eq!(game.sim.world.chunk_count(), 0);
}

#[test]
fn arrows_and_wasd_push_the_same_way_and_opposites_cancel() {
    assert_eq!(Game::key_push(&[Key::ArrowLeft]), Vec2::new(-1.0, 0.0));
    assert_eq!(Game::key_push(&[Key::Char('a')]), Vec2::new(-1.0, 0.0));
    assert_eq!(Game::key_push(&[Key::Char('W'), Key::ArrowRight]), Vec2::new(1.0, -1.0));
    assert_eq!(Game::key_push(&[Key::Char('s'), Key::ArrowUp]), Vec2::ZERO);
    assert_eq!(Game::key_push(&[Key::Char('x'), Key::Space]), Vec2::ZERO);
}

#[path = "app_play_tests.rs"]
mod play;

#[path = "app_frame_tests.rs"]
mod frame;

#[path = "app_title_tests.rs"]
mod titles;
