//! The bank and the right-click menus, driven with no window.

use sim::{PersonKind, Slot};

use super::fights::{body_of, until, with_goblin};
use super::frame::{GRASS, body, count, snap};
use super::people::with_person;
use super::play::{pass, playing, press_tile, screen_of};
use super::*;
use crate::bank_panel;

const COPPER: [u8; 3] = [0xc7, 0x82, 0x46];

/// The game with a bank chest two tiles east of the player, who has
/// walked up to it: the bank is open.
fn at_the_bank(name: &str) -> (Harness, Shell<Game>, Game, (i32, i32)) {
    let (mut h, mut shell, mut game) = playing(name);
    game.sim.monsters.clear();
    let (x, y) = game.sim.player.tile;
    let chest = (x + 2, y);
    assert!(game.sim.world.place_prop(PropKind::BankChest, chest.0, chest.1));
    turn(&mut h, &mut shell, &mut game);
    press_tile(&mut h, &mut shell, &mut game, chest);
    until(&mut h, &mut shell, &mut game, 10.0, |game| game.sim.banking());
    pass(&mut h, &mut shell, &mut game, 0.6);
    assert!(game.sim.banking());
    (h, shell, game, chest)
}

fn bag_slot(h: &Harness, shell: &Shell<Game>, slot: usize) -> Vec2 {
    h.rect_of(body(shell, View::World).with("slot").with_index(slot)).expect("a slot of the bag").center()
}

fn bank_slot(h: &Harness, shell: &Shell<Game>, slot: usize) -> Rect {
    h.rect_of(body(shell, View::World).with("banked").with_index(slot)).expect("a slot of the bank")
}

/// Press the right button at a point of the window.
fn right_click(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, at: Vec2) {
    h.move_to(at);
    turn(h, shell, game);
    h.right_press();
    turn(h, shell, game);
    turn(h, shell, game);
}

/// Where a row of the open right-click menu is, by what it says.
fn menu_row(h: &Harness, label: &str) -> Option<Vec2> {
    let rows = WidgetId::ROOT.with("popup").with("context").with_index(0).with("items");
    (0..24).find_map(|i| h.rect_of(rows.with_index(i).with(label))).map(|rect| rect.center())
}

/// Right-click at a point and pick a row of the menu that opens.
fn pick(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, at: Vec2, label: &str) {
    right_click(h, shell, game, at);
    assert!(shell.popup_open(), "a menu opened");
    let row = menu_row(h, label).unwrap_or_else(|| panic!("the menu has no row saying {label}"));
    click(h, shell, game, row);
    turn(h, shell, game);
    assert!(!shell.popup_open(), "picking closed the menu");
}

#[test]
fn walking_up_to_the_chest_opens_the_bank_with_the_bag_beside_it() {
    let (mut h, mut shell, mut game, chest) = at_the_bank("bank_open");
    assert_eq!(game.sim.using(), Some(chest), "the chest keeps its ring while the bank is open");
    assert!(game.hud.bag_open, "the bag opened by itself");
    let panel = h.rect_of(body(&shell, View::World).with("bank")).expect("the bank's panel");
    let bag = h.rect_of(body(&shell, View::World).with("slot").with_index(0)).expect("the bag");
    assert!(panel.max.x < bag.min.x, "side by side: {panel:?} and {bag:?}");
    assert!(bank_slot(&h, &shell, 0).width() >= 70.0 && bank_slot(&h, &shell, 47).max.y < panel.max.y, "48 big slots, all inside");
    // Esc steps back from the bank before it leaves the game.
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert!(!game.sim.banking() && game.mode == Mode::Playing { slot: 0 });
    assert_eq!(game.sim.using(), None);
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert_eq!(game.mode, Mode::Title);
}

#[test]
fn a_press_moves_one_each_way_and_it_is_kept_at_once() {
    let (mut h, mut shell, mut game, _) = at_the_bank("bank_one");
    game.sim.inventory.add(Item::CopperOre, 5);
    game.sim.inventory.add(Item::HealingPotion, 2);
    turn(&mut h, &mut shell, &mut game);
    let ore = bag_slot(&h, &shell, 0);
    click(&mut h, &mut shell, &mut game, ore);
    click(&mut h, &mut shell, &mut game, ore);
    assert_eq!((game.sim.inventory.count(Item::CopperOre), game.sim.bank.count(Item::CopperOre)), (3, 2));
    // At the bank a press on a potion banks it: nobody drinks it.
    let potion = bag_slot(&h, &shell, 1);
    click(&mut h, &mut shell, &mut game, potion);
    assert_eq!((game.sim.inventory.count(Item::HealingPotion), game.sim.bank.count(Item::HealingPotion)), (1, 1));
    assert_eq!(game.sim.player.goal(), None, "and nobody walked anywhere");
    let saved = game.saves.read(0).expect("a save").expect("that reads");
    assert_eq!((saved.bank.count(Item::CopperOre), saved.bank.count(Item::HealingPotion), saved.inventory.count(Item::CopperOre)), (2, 1, 3));
    // And a press on the bank takes one out.
    let banked = bank_slot(&h, &shell, 0).center();
    click(&mut h, &mut shell, &mut game, banked);
    assert_eq!((game.sim.inventory.count(Item::CopperOre), game.sim.bank.count(Item::CopperOre)), (4, 1));
    assert_eq!(game.saves.read(0).unwrap().unwrap().bank.count(Item::CopperOre), 1);
}

#[test]
fn a_right_press_offers_how_many_and_x_asks_for_a_number() {
    let (mut h, mut shell, mut game, _) = at_the_bank("bank_menu");
    game.sim.inventory.add(Item::CopperOre, 40);
    turn(&mut h, &mut shell, &mut game);
    let ore = bag_slot(&h, &shell, 0);
    right_click(&mut h, &mut shell, &mut game, ore);
    for label in ["Put 1 in", "Put 5 in", "Put 10 in", "Put X in…", "Put all in"] {
        assert!(menu_row(&h, label).is_some(), "no row saying {label}");
    }
    assert_eq!(menu_row(&h, "Wear"), None, "ore is not worn");
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert!(!shell.popup_open() && game.sim.banking(), "Esc closed the menu, not the bank");

    pick(&mut h, &mut shell, &mut game, ore, "Put 10 in");
    assert_eq!(game.sim.bank.count(Item::CopperOre), 10);
    // X asks how many, takes digits only, and is remembered.
    right_click(&mut h, &mut shell, &mut game, ore);
    let row = menu_row(&h, "Put X in…").unwrap();
    click(&mut h, &mut shell, &mut game, row);
    turn(&mut h, &mut shell, &mut game);
    assert!(shell.popup_open(), "the dialog that asks how many");
    h.type_text("1x7");
    turn(&mut h, &mut shell, &mut game);
    key(&mut h, &mut shell, &mut game, Key::Enter, Modifiers::NONE);
    assert!(!shell.popup_open());
    assert_eq!((game.sim.bank.count(Item::CopperOre), game.sim.inventory.count(Item::CopperOre)), (27, 13));
    right_click(&mut h, &mut shell, &mut game, ore);
    assert!(menu_row(&h, "Put 17 in").is_some(), "the number last typed is offered again");
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);

    // Out of the bank the same way.
    let banked = bank_slot(&h, &shell, 0).center();
    pick(&mut h, &mut shell, &mut game, banked, "Take 5 out");
    assert_eq!((game.sim.bank.count(Item::CopperOre), game.sim.inventory.count(Item::CopperOre)), (22, 18));
    pick(&mut h, &mut shell, &mut game, banked, "Take all out");
    assert_eq!((game.sim.bank.count(Item::CopperOre), game.sim.inventory.count(Item::CopperOre)), (0, 40));
    assert_eq!(game.saves.read(0).unwrap().unwrap().inventory.count(Item::CopperOre), 40, "kept at once");
}

#[test]
fn one_button_puts_the_whole_bag_in() {
    let (mut h, mut shell, mut game, _) = at_the_bank("bank_all");
    game.sim.inventory.add(Item::CopperOre, 9);
    game.sim.inventory.add(Item::BronzeSword, 2);
    game.sim.inventory.add(Item::StrengthPotion, 3);
    turn(&mut h, &mut shell, &mut game);
    let button = h.rect_of(body(&shell, View::World).with(bank_panel::PUT_BAG)).expect("the button");
    assert!(button.width() >= 250.0 && button.height() >= 45.0, "a big one: {button:?}");
    click(&mut h, &mut shell, &mut game, button.center());
    assert!(game.sim.inventory.is_empty());
    assert_eq!((game.sim.bank.count(Item::CopperOre), game.sim.bank.count(Item::BronzeSword), game.sim.bank.count(Item::StrengthPotion)), (9, 2, 3));
    assert_eq!(game.sim.bank.kinds(), 3, "three kinds, three slots");
    assert_eq!(game.saves.read(0).unwrap().unwrap().bank.count(Item::BronzeSword), 2);
}

#[test]
fn away_from_the_bank_a_right_press_on_the_bag_offers_to_wear_or_drink() {
    let (mut h, mut shell, mut game) = playing("menu_bag");
    game.sim.monsters.clear();
    game.sim.inventory.add(Item::BronzeSword, 1);
    game.sim.inventory.add(Item::StrengthPotion, 1);
    game.sim.inventory.add(Item::CopperOre, 3);
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('g'), Modifiers::NONE);
    let (sword, potion, ore) = (bag_slot(&h, &shell, 0), bag_slot(&h, &shell, 1), bag_slot(&h, &shell, 2));
    right_click(&mut h, &mut shell, &mut game, sword);
    assert!(menu_row(&h, "Wear").is_some() && menu_row(&h, "Put 1 in").is_none(), "no banking away from the bank");
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    pick(&mut h, &mut shell, &mut game, sword, "Wear");
    assert_eq!(game.sim.gear.worn(Slot::Weapon), Some(Item::BronzeSword));
    pick(&mut h, &mut shell, &mut game, potion, "Drink");
    assert_eq!(game.sim.inventory.count(Item::StrengthPotion), 0);
    // There is nothing to do with ore out here: no menu at all.
    right_click(&mut h, &mut shell, &mut game, ore);
    assert!(!shell.popup_open());
    // What is worn can be taken off from its own menu.
    let worn = h.rect_of(body(&shell, View::World).with("gear").with(Slot::Weapon.name())).expect("the weapon's slot").center();
    pick(&mut h, &mut shell, &mut game, worn, "Take off");
    assert_eq!((game.sim.gear.worn(Slot::Weapon), game.sim.inventory.count(Item::BronzeSword)), (None, 1));
    assert_eq!(game.saves.read(0).unwrap().unwrap().gear.worn(Slot::Weapon), None, "kept at once");
}

#[test]
fn a_right_press_on_the_world_lists_what_can_be_done_there() {
    let (mut h, mut shell, mut game) = playing("menu_world");
    game.sim.monsters.clear();
    let (x, y) = game.sim.player.tile;
    let rock = (x + 3, y);
    assert!(game.sim.world.place_prop(PropKind::Rock, rock.0, rock.1));
    turn(&mut h, &mut shell, &mut game);
    // Bare grass: walking there is all there is.
    let grass = screen_of(&game, (x, y + 3));
    right_click(&mut h, &mut shell, &mut game, grass);
    assert!(menu_row(&h, "Walk here").is_some() && menu_row(&h, "Mine Copper rock").is_none());
    assert_eq!(game.sim.player.goal(), None, "a right press alone does nothing");
    let row = menu_row(&h, "Walk here").unwrap();
    click(&mut h, &mut shell, &mut game, row);
    assert_eq!(game.sim.player.goal(), Some((x, y + 3)));
    pass(&mut h, &mut shell, &mut game, 3.0);

    // A rock: mining it, or just walking up to it.
    let at = screen_of(&game, rock);
    pick(&mut h, &mut shell, &mut game, at, "Walk here");
    assert!(game.sim.player.busy() && game.sim.using().is_none(), "walking, not mining");
    pass(&mut h, &mut shell, &mut game, 4.0);
    assert_eq!(game.sim.inventory.count(Item::CopperOre), 0);
    let at = screen_of(&game, rock);
    pick(&mut h, &mut shell, &mut game, at, "Mine Copper rock");
    assert_eq!(game.sim.using(), Some(rock));
}

#[test]
fn a_right_press_on_somebody_offers_to_fight_or_to_talk() {
    let (mut h, mut shell, mut game) = with_goblin("menu_goblin", 6);
    let at = body_of(&game, 0);
    pick(&mut h, &mut shell, &mut game, at, "Attack Goblin");
    assert_eq!(game.sim.target(), Some(0));

    let (mut h, mut shell, mut game) = with_person("menu_person", PersonKind::Banker, 4);
    let at = game.camera.to_screen(game.person_grounds()[0] - Vec2::new(0.0, 0.5), game.view, game.scale);
    right_click(&mut h, &mut shell, &mut game, at);
    assert!(menu_row(&h, "Talk to Tilda").is_some() && menu_row(&h, "Walk here").is_some());
    let row = menu_row(&h, "Talk to Tilda").unwrap();
    click(&mut h, &mut shell, &mut game, row);
    assert_eq!(game.sim.awaited(), Some(0));
    until(&mut h, &mut shell, &mut game, 10.0, |game| game.sim.saying().is_some());
    assert_eq!(game.sim.saying().map(|s| s.kind), Some(PersonKind::Banker));
}

#[test]
fn the_bank_and_a_menu_are_drawn_over_the_world() {
    let (mut h, mut shell, mut game, _) = at_the_bank("frame_bank");
    game.sim.bank.put(Item::CopperOre, 123);
    game.sim.bank.put(Item::HealingPotion, 4);
    game.sim.bank.put(Item::BronzeSword, 2);
    game.sim.inventory.add(Item::TinOre, 7);
    turn(&mut h, &mut shell, &mut game);
    let Some(image) = snap("frame-bank", &mut h, &mut shell, &mut game) else { return };
    let panel = h.rect_of(body(&shell, View::World).with("bank")).expect("the bank's panel");
    assert_eq!(count(&image, panel.shrink(4.0), GRASS, 1), 0, "no grass shows through the bank");
    assert!(count(&image, bank_slot(&h, &shell, 0), COPPER, 12) > 40, "the ore's picture is in the first slot");
    assert_eq!(count(&image, bank_slot(&h, &shell, 5), COPPER, 12), 0);
    // A menu over it all.
    let banked = bank_slot(&h, &shell, 0).center();
    right_click(&mut h, &mut shell, &mut game, banked);
    assert!(shell.popup_open());
    let Some(image) = snap("frame-bank-menu", &mut h, &mut shell, &mut game) else { return };
    let row = menu_row(&h, "Take all out").expect("the last row");
    let rows = Rect::from_center_size(row, Vec2::new(200.0, 40.0));
    assert!(count(&image, rows, [242, 242, 244], 14) > 100, "the row's words: {}", count(&image, rows, [242, 242, 244], 14));
}

#[test]
fn the_chest_and_the_village_scenery_stand_in_the_world_and_are_in_the_way() {
    let (mut h, mut shell, mut game) = playing("frame_village");
    game.sim.monsters.clear();
    let (x, y) = game.sim.player.tile;
    let props = [(PropKind::BankChest, (x - 3, y - 1)), (PropKind::Crates, (x - 1, y - 2)), (PropKind::Barrel, (x + 1, y - 2)), (PropKind::Signpost, (x + 3, y - 1)), (PropKind::Well, (x + 3, y + 2)), (PropKind::Fence, (x - 2, y + 2)), (PropKind::Fence, (x - 1, y + 2)), (PropKind::Fence, (x, y + 2))];
    for (kind, (px, py)) in props {
        game.sim.world.remove_prop(px, py);
        assert!(game.sim.world.place_prop(kind, px, py), "{kind:?}");
        assert!(!game.sim.world.walkable(px, py), "{kind:?} is in the way");
        assert!(game.look.prop(kind).is_some(), "{kind:?} has a drawing");
    }
    turn(&mut h, &mut shell, &mut game);
    let Some(image) = snap("frame-village", &mut h, &mut shell, &mut game) else { return };
    let px = game.camera.tile_px(game.scale);
    for (kind, tile) in props {
        // Something other than grass is drawn where each stands.
        let at = Rect::from_center_size(screen_of(&game, tile) - Vec2::new(0.0, px * 0.3), Vec2::new(px * 0.8, px * 0.8));
        assert!(count(&image, at, GRASS, 1) < (at.area() * 0.6) as usize, "{kind:?} is drawn: {} of {}", count(&image, at, GRASS, 1), at.area());
    }
    // Fence pieces side by side join: the rail runs unbroken across the
    // line between two tiles.
    const RAIL: [u8; 3] = [0x86, 0x60, 0x39];
    let seam = screen_of(&game, (x - 1, y + 2)) - Vec2::new(px * 0.5, 0.0);
    let across = Rect::from_center_size(seam - Vec2::new(0.0, px * 0.4), Vec2::new(6.0, px * 0.5));
    assert!(count(&image, across, RAIL, 4) > 30, "the rails cross the seam: {}", count(&image, across, RAIL, 4));
}

#[test]
fn the_editors_props_tab_lists_the_chest_with_its_picture_under_stations() {
    let (mut h, mut shell, mut game) = playing("frame_editor_props");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    panel_tab(&mut h, &mut shell, &mut game, 1);
    let Some(image) = snap("frame-editor-props", &mut h, &mut shell, &mut game) else { return };
    // The chest's gold bands are in its row, and not in the anvil's.
    const GOLD: [u8; 3] = [0xc1, 0x9b, 0x4a];
    let row = |kind| h.rect_of(panel_id(&shell, &[panel::Group::of(kind).label(), panel::prop_label(kind)])).expect("a row for it");
    assert!(count(&image, row(PropKind::BankChest), GOLD, 10) > 20, "the chest's picture: {}", count(&image, row(PropKind::BankChest), GOLD, 10));
    assert_eq!(count(&image, row(PropKind::Anvil), GOLD, 10), 0);
    let click_at = row(PropKind::BankChest).center();
    click(&mut h, &mut shell, &mut game, click_at);
    assert_eq!((game.editor.tool, game.editor.prop), (Tool::Place, PropKind::BankChest));
}
