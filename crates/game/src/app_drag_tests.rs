//! Carrying things from slot to slot with the pointer, driven with no
//! window.

use sim::Slot;
use sim::items::Stack;

use super::bank::{at_the_bank, bag_slot, bank_slot};
use super::frame::{body, count, snap};
use super::play::playing;
use super::*;

const COPPER: [u8; 3] = [0xc7, 0x82, 0x46];

/// Press at one point of the window, carry to another by way of the
/// middle, and let go.
fn carry(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, from: Vec2, to: Vec2) {
    h.move_to(from);
    turn(h, shell, game);
    h.press();
    turn(h, shell, game);
    h.move_to(from.lerp(to, 0.5));
    turn(h, shell, game);
    h.move_to(to);
    turn(h, shell, game);
    h.release();
    turn(h, shell, game);
    turn(h, shell, game);
}

fn gear_slot(h: &Harness, shell: &Shell<Game>, slot: Slot) -> Vec2 {
    h.rect_of(body(shell, View::World).with("gear").with(slot.name())).expect("a slot of the gear").center()
}

/// What a slot of the bag holds, and how many.
fn in_bag(game: &Game, slot: usize) -> Option<(Item, u32)> {
    game.sim.inventory.slots()[slot].map(|Stack { item, count }| (item, count))
}

fn in_bank(game: &Game, slot: usize) -> Option<(Item, u32)> {
    game.sim.bank.slots()[slot].map(|Stack { item, count }| (item, count))
}

/// The game with ore and a sword in the bag, and the bag and the gear
/// open.
fn packed(name: &str) -> (Harness, Shell<Game>, Game) {
    let (mut h, mut shell, mut game) = playing(name);
    game.sim.monsters.clear();
    game.sim.inventory.add(Item::CopperOre, 12);
    game.sim.inventory.add(Item::BronzeSword, 1);
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('g'), Modifiers::NONE);
    (h, shell, game)
}

#[test]
fn things_in_the_bag_are_carried_from_slot_to_slot() {
    let (mut h, mut shell, mut game) = packed("drag_bag");
    let slot = |h: &Harness, shell: &Shell<Game>, i| bag_slot(h, shell, i);
    // Onto an empty slot: it moves there.
    let (from, to) = (slot(&h, &shell, 0), slot(&h, &shell, 7));
    carry(&mut h, &mut shell, &mut game, from, to);
    assert_eq!((in_bag(&game, 0), in_bag(&game, 7)), (None, Some((Item::CopperOre, 12))));
    // Onto a slot that holds something: the two trade places.
    let from = slot(&h, &shell, 1);
    carry(&mut h, &mut shell, &mut game, from, to);
    assert_eq!((in_bag(&game, 1), in_bag(&game, 7)), (Some((Item::CopperOre, 12)), Some((Item::BronzeSword, 1))));
    assert_eq!(game.sim.gear.worn(Slot::Weapon), None, "carrying the sword about did not put it on");
    let saved = game.saves.read(0).expect("a save").expect("that reads");
    assert_eq!(saved.inventory.slots()[7].map(|s| s.item), Some(Item::BronzeSword), "the bag is kept as it was left");
    // Carried off and brought back to where it was: nothing, and no press either.
    let (sword, elsewhere) = (slot(&h, &shell, 7), slot(&h, &shell, 20));
    h.move_to(sword);
    turn(&mut h, &mut shell, &mut game);
    h.press();
    turn(&mut h, &mut shell, &mut game);
    h.move_to(elsewhere);
    turn(&mut h, &mut shell, &mut game);
    assert!(game.hud.drag.carrying(), "it is in the air");
    h.move_to(sword);
    turn(&mut h, &mut shell, &mut game);
    h.release();
    turn(&mut h, &mut shell, &mut game);
    assert!(!game.hud.drag.carrying());
    assert_eq!((in_bag(&game, 7), game.sim.gear.worn(Slot::Weapon)), (Some((Item::BronzeSword, 1)), None));
    // A press that goes nowhere is still a press: the sword goes on.
    click(&mut h, &mut shell, &mut game, sword);
    assert_eq!(game.sim.gear.worn(Slot::Weapon), Some(Item::BronzeSword));
}

#[test]
fn let_go_anywhere_but_a_slot_it_goes_back_and_nobody_walks() {
    let (mut h, mut shell, mut game) = packed("drag_nowhere");
    let (from, tile) = (bag_slot(&h, &shell, 0), game.sim.player.tile);
    carry(&mut h, &mut shell, &mut game, from, Vec2::new(300.0, 300.0));
    assert_eq!(in_bag(&game, 0), Some((Item::CopperOre, 12)));
    assert_eq!((game.sim.player.tile, game.sim.player.goal()), (tile, None), "letting go over the world is not a press on it");
}

#[test]
fn gear_goes_on_when_dropped_on_the_gear_and_comes_off_when_carried_to_the_bag() {
    let (mut h, mut shell, mut game) = packed("drag_gear");
    // Let go over any slot of the gear, the sword goes where swords go.
    let (sword, head) = (bag_slot(&h, &shell, 1), gear_slot(&h, &shell, Slot::Head));
    carry(&mut h, &mut shell, &mut game, sword, head);
    assert_eq!((game.sim.gear.worn(Slot::Weapon), in_bag(&game, 1)), (Some(Item::BronzeSword), None));
    assert_eq!(game.saves.read(0).unwrap().unwrap().gear.worn(Slot::Weapon), Some(Item::BronzeSword), "kept at once");
    // Ore is not worn, wherever it is dropped.
    let ore = bag_slot(&h, &shell, 0);
    carry(&mut h, &mut shell, &mut game, ore, head);
    assert_eq!((in_bag(&game, 0), game.sim.gear.worn(Slot::Head)), (Some((Item::CopperOre, 12)), None));
    // And off again, into the slot it is carried to.
    let (worn, to) = (gear_slot(&h, &shell, Slot::Weapon), bag_slot(&h, &shell, 14));
    carry(&mut h, &mut shell, &mut game, worn, to);
    assert_eq!((game.sim.gear.worn(Slot::Weapon), in_bag(&game, 14)), (None, Some((Item::BronzeSword, 1))));
}

#[test]
fn things_are_carried_between_the_bag_and_the_bank_and_about_the_bank() {
    let (mut h, mut shell, mut game, _) = at_the_bank("drag_bank");
    game.sim.inventory.add(Item::CopperOre, 12);
    game.sim.inventory.add(Item::HealingPotion, 2);
    turn(&mut h, &mut shell, &mut game);
    // The whole stack, into the slot of the bank it is dropped on.
    let (ore, to) = (bag_slot(&h, &shell, 0), bank_slot(&h, &shell, 10).center());
    carry(&mut h, &mut shell, &mut game, ore, to);
    assert_eq!((in_bag(&game, 0), in_bank(&game, 10)), (None, Some((Item::CopperOre, 12))));
    // One potion carried is one potion banked.
    let potion = bag_slot(&h, &shell, 1);
    carry(&mut h, &mut shell, &mut game, potion, to);
    assert_eq!((game.sim.inventory.count(Item::HealingPotion), in_bank(&game, 0)), (1, Some((Item::HealingPotion, 1))));
    // About the bank: onto a free slot, and onto a taken one.
    let (from, free) = (bank_slot(&h, &shell, 10).center(), bank_slot(&h, &shell, 30).center());
    carry(&mut h, &mut shell, &mut game, from, free);
    assert_eq!((in_bank(&game, 10), in_bank(&game, 30)), (None, Some((Item::CopperOre, 12))));
    let potions = bank_slot(&h, &shell, 0).center();
    carry(&mut h, &mut shell, &mut game, free, potions);
    assert_eq!((in_bank(&game, 0), in_bank(&game, 30)), (Some((Item::CopperOre, 12)), Some((Item::HealingPotion, 1))));
    // Back out, into the slot of the bag it is dropped on.
    let (ore, to) = (bank_slot(&h, &shell, 0).center(), bag_slot(&h, &shell, 22));
    carry(&mut h, &mut shell, &mut game, ore, to);
    assert_eq!((in_bank(&game, 0), in_bag(&game, 22)), (None, Some((Item::CopperOre, 12))));
    let saved = game.saves.read(0).unwrap().unwrap();
    assert_eq!((saved.inventory.slots()[22].map(|s| s.count), saved.bank.slots()[30].map(|s| s.item)), (Some(12), Some(Item::HealingPotion)), "kept at once");
    // A press that barely moves is still a press: one goes in.
    let ore = bag_slot(&h, &shell, 22);
    h.move_to(ore);
    turn(&mut h, &mut shell, &mut game);
    h.press();
    turn(&mut h, &mut shell, &mut game);
    h.move_to(ore + Vec2::new(3.0, 2.0));
    turn(&mut h, &mut shell, &mut game);
    h.release();
    turn(&mut h, &mut shell, &mut game);
    assert_eq!((game.sim.inventory.count(Item::CopperOre), game.sim.bank.count(Item::CopperOre)), (11, 1));
}

#[test]
fn what_is_carried_is_drawn_under_the_pointer_and_faint_where_it_was() {
    let (mut h, mut shell, mut game) = packed("frame_drag");
    let slot = h.rect_of(body(&shell, View::World).with("slot").with_index(0)).expect("the first slot");
    let Some(before) = snap("frame-drag-before", &mut h, &mut shell, &mut game) else { return };
    let whole = count(&before, slot, COPPER, 12);
    assert!(whole > 40, "the ore's picture is in its slot");
    let over = Vec2::new(700.0, 300.0);
    h.move_to(slot.center());
    turn(&mut h, &mut shell, &mut game);
    h.press();
    turn(&mut h, &mut shell, &mut game);
    h.move_to(over);
    turn(&mut h, &mut shell, &mut game);
    assert!(game.hud.drag.carrying());
    let Some(image) = snap("frame-drag", &mut h, &mut shell, &mut game) else { return };
    let carried = Rect::from_center_size(over, Vec2::new(80.0, 80.0));
    assert!(count(&image, carried, COPPER, 12) > 40, "the ore is under the pointer: {}", count(&image, carried, COPPER, 12));
    assert_eq!(count(&before, carried, COPPER, 12), 0);
    assert!(count(&image, slot, COPPER, 12) < whole / 4, "and only a ghost of it is left in its slot: {} of {whole}", count(&image, slot, COPPER, 12));
    h.release();
    turn(&mut h, &mut shell, &mut game);
}
