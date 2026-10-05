//! Smelting and forging, driven with no window: the stations, the
//! panel that opens at them, and what its buttons do.

use sim::{RECIPES, Station};

use super::fights::until;
use super::frame::{body, count, snap};
use super::play::{pass, playing, press_tile, screen_of};
use super::*;

const BRONZE: [u8; 3] = [0xb8, 0x7a, 0x3d];
const STONE: [u8; 3] = [0x8a, 0x86, 0x85];
const TIN: [u8; 3] = [0xdf, 0xe3, 0xe6];

/// The game with a furnace two tiles east of the player and an anvil
/// two tiles west, and the player a smith good enough for everything.
fn smithy(name: &str) -> (Harness, Shell<Game>, Game, Stations) {
    let (mut h, mut shell, mut game) = playing(name);
    game.sim.monsters.clear();
    let (x, y) = game.sim.player.tile;
    let at = Stations { furnace: (x + 2, y), anvil: (x - 2, y) };
    assert!(game.sim.world.place_prop(PropKind::Furnace, at.furnace.0, at.furnace.1));
    assert!(game.sim.world.place_prop(PropKind::Anvil, at.anvil.0, at.anvil.1));
    game.sim.skills.set_xp(Skill::Smithing, sim::skills::xp_for_level(5));
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game, at)
}

/// The tiles the smithy's stations stand on.
struct Stations {
    furnace: (i32, i32),
    anvil: (i32, i32),
}

fn panel(h: &Harness, shell: &Shell<Game>) -> Option<Rect> {
    h.rect_of(body(shell, View::World).with("making"))
}

/// The button of a recipe's row that makes `label` of it.
fn make_button(h: &Harness, shell: &Shell<Game>, makes: Item, label: &str) -> Option<Rect> {
    let index = RECIPES.iter().position(|r| r.makes == makes).expect("a recipe");
    h.rect_of(body(shell, View::World).with("make").with_index(index).with(label))
}

/// Walk up to a station and wait until the player is at it.
fn go_to(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, tile: (i32, i32)) {
    press_tile(h, shell, game, tile);
    until(h, shell, game, 10.0, |game| game.sim.station().is_some());
    pass(h, shell, game, 0.1);
}

#[test]
fn clicking_a_furnace_walks_up_and_opens_its_panel_and_walking_off_closes_it() {
    let (mut h, mut shell, mut game, Stations { furnace, .. }) = smithy("smith_open");
    assert_eq!(panel(&h, &shell), None);
    go_to(&mut h, &mut shell, &mut game, furnace);
    assert_eq!(game.sim.station(), Some(Station::Furnace));
    let open = panel(&h, &shell).expect("the panel");
    assert!(game.view.contains(open.min) && game.view.contains(open.max));
    assert!(open.width() >= 500.0, "big enough to read");
    // The furnace smelts bars and forges nothing.
    assert!(make_button(&h, &shell, Item::BronzeBar, "All").is_some());
    assert!(make_button(&h, &shell, Item::BronzeSword, "1").is_none());
    // Walking off closes it.
    let (x, y) = game.sim.player.tile;
    press_tile(&mut h, &mut shell, &mut game, (x - 1, y + 2));
    pass(&mut h, &mut shell, &mut game, 0.1);
    assert_eq!(game.sim.station(), None);
    turn(&mut h, &mut shell, &mut game);
    assert_eq!(panel(&h, &shell), None);
}

#[test]
fn the_buttons_make_one_five_or_all_and_a_click_on_the_panel_walks_nowhere() {
    let (mut h, mut shell, mut game, Stations { furnace, .. }) = smithy("smith_buttons");
    game.sim.inventory.add(Item::CopperOre, 12);
    game.sim.inventory.add(Item::TinOre, 9);
    go_to(&mut h, &mut shell, &mut game, furnace);
    let stood = game.sim.player.tile;
    let made = |h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, label: &str| {
        let at = make_button(h, shell, Item::BronzeBar, label).expect("a button").center();
        click(h, shell, game, at);
        assert!(game.sim.making().is_some(), "making, after {label}");
        until(h, shell, game, 60.0, |game| game.sim.making().is_none());
        pass(h, shell, game, 0.5);
        game.sim.inventory.count(Item::BronzeBar)
    };
    assert_eq!(made(&mut h, &mut shell, &mut game, "1"), 1);
    assert_eq!(made(&mut h, &mut shell, &mut game, "5"), 6);
    assert_eq!(made(&mut h, &mut shell, &mut game, "All"), 9, "all the tin there was");
    assert_eq!((game.sim.inventory.count(Item::CopperOre), game.sim.inventory.count(Item::TinOre)), (3, 0));
    assert_eq!(game.sim.player.tile, stood, "they never left the furnace");
    assert_eq!(game.sim.station(), Some(Station::Furnace), "and are still at it");
    // With no tin left the buttons take no presses.
    let at = make_button(&h, &shell, Item::BronzeBar, "1").expect("still drawn").center();
    click(&mut h, &mut shell, &mut game, at);
    assert!(game.sim.making().is_none());
    assert!(game.saves.read(0).unwrap().unwrap().inventory.count(Item::BronzeBar) == 9, "what was made is saved");
}

#[test]
fn the_anvil_lists_the_whole_set_and_only_what_the_smith_is_up_to_can_be_made() {
    let (mut h, mut shell, mut game, Stations { anvil, .. }) = smithy("smith_anvil");
    game.sim.skills.set_xp(Skill::Smithing, sim::skills::xp_for_level(3));
    game.sim.inventory.add(Item::BronzeBar, 20);
    go_to(&mut h, &mut shell, &mut game, anvil);
    assert_eq!(game.sim.station(), Some(Station::Anvil));
    let rows = panel(&h, &shell).expect("the panel");
    assert!(rows.max.y <= game.view.max.y, "all eight rows fit the window");
    for recipe in RECIPES.iter().filter(|r| r.station == Station::Anvil) {
        let there = make_button(&h, &shell, recipe.makes, "1").is_some();
        assert_eq!(there, recipe.level <= 3, "{:?}", recipe.makes);
    }
    let at = make_button(&h, &shell, Item::BronzeHelmet, "1").unwrap().center();
    click(&mut h, &mut shell, &mut game, at);
    // The player hammers once for the thing made, facing the anvil.
    let mut poses = Vec::new();
    for _ in 0..80 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        poses.push(super::play::pose_of(&game).unwrap());
    }
    assert_eq!(game.sim.inventory.count(Item::BronzeHelmet), 1);
    assert_eq!(super::play::runs(&poses, Pose::WorkB), 1, "one strike for the one helmet");
    assert_eq!(game.facing, Facing::Left, "facing the anvil");
}

#[test]
fn escape_steps_back_from_a_station_before_it_leaves_the_game() {
    let (mut h, mut shell, mut game, Stations { furnace, .. }) = smithy("smith_escape");
    go_to(&mut h, &mut shell, &mut game, furnace);
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert_eq!(game.sim.station(), None);
    assert!(matches!(game.mode, Mode::Playing { .. }), "still in the game");
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert_eq!(game.mode, Mode::Title);
}

#[test]
fn the_editor_lists_the_new_props_and_places_them() {
    let (mut h, mut shell, mut game) = playing("smith_editor");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    panel_tab(&mut h, &mut shell, &mut game, 1);
    let (x, y) = game.sim.player.tile;
    for (label, kind, tile) in [("Tin rock", PropKind::TinRock, (x + 2, y)), ("Furnace", PropKind::Furnace, (x + 2, y + 2)), ("Anvil", PropKind::Anvil, (x - 2, y + 2))] {
        let row = prop_row(&h, &shell, kind);
        click(&mut h, &mut shell, &mut game, row);
        assert_eq!((game.editor.tool, game.editor.prop), (Tool::Place, kind));
        let at = screen_of(&game, tile);
        click(&mut h, &mut shell, &mut game, at);
        assert_eq!(game.sim.world.prop_at(tile.0, tile.1).map(|p| p.kind), Some(kind), "{label}");
    }
}

#[test]
fn the_stations_and_the_tin_rock_are_drawn_and_the_panel_shows_what_things_take() {
    let (mut h, mut shell, mut game, Stations { furnace, anvil }) = smithy("frame_smithy");
    let (x, y) = game.sim.player.tile;
    assert!(game.sim.world.place_prop(PropKind::TinRock, x, y - 2));
    game.sim.inventory.add(Item::BronzeBar, 4);
    game.sim.inventory.add(Item::BronzePickaxe, 1);
    game.sim.skills.set_xp(Skill::Smithing, sim::skills::xp_for_level(3));
    go_to(&mut h, &mut shell, &mut game, anvil);
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    let Some(image) = snap("frame-smithy", &mut h, &mut shell, &mut game) else { return };
    let px = game.camera.tile_px(game.scale);
    let over = |tile: (i32, i32)| Rect::from_center_size(screen_of(&game, tile) - Vec2::new(0.0, px * 0.5), Vec2::new(px * 1.2, px * 1.6));
    assert!(count(&image, over(furnace), STONE, 3) > 2000, "the furnace's stone: {}", count(&image, over(furnace), STONE, 3));
    assert!(count(&image, over((x, y - 2)), TIN, 3) > 150, "the tin in the rock: {}", count(&image, over((x, y - 2)), TIN, 3));
    // The panel: bronze pictures down its left, and every row's needs.
    let open = panel(&h, &shell).expect("the panel");
    assert!(count(&image, open, BRONZE, 6) > 1500, "the things to make: {}", count(&image, open, BRONZE, 6));
    assert!(count(&image, open, [0xc9, 0xb6, 0x8a], 2) > 5000, "each on its light plate");
    // What cannot be afforded says so in red; what the smith is not up
    // to names the level.
    assert!(count(&image, open, [0xa3, 0x28, 0x1c], 12) > 60, "red for what is lacking: {}", count(&image, open, [0xa3, 0x28, 0x1c], 12));
}
