//! Tests for the bank: what it holds, and moving things in and out.

use super::*;
use crate::items::Inventory;
use crate::save::{Character, Saved};
use crate::terrain::Terrain;
use crate::world::World;

/// A small field with a bank chest at (5, 2) and the player two tiles
/// west of it.
fn by_the_chest() -> Sim {
    let mut world = World::new();
    for y in 0..6 {
        for x in 0..12 {
            world.set_terrain(x, y, Terrain::Grass);
        }
    }
    world.place_prop(PropKind::BankChest, 5, 2);
    let mut sim = Sim::new(world);
    sim.player.tile = (3, 2);
    sim.player.from = (3, 2);
    sim
}

/// The same, with the player walked up to the chest.
fn banking() -> Sim {
    let mut sim = by_the_chest();
    sim.click((5, 2));
    for _ in 0..20 {
        sim.tick();
    }
    assert!(sim.banking());
    sim.take_events();
    sim
}

#[test]
fn everything_stacks_and_a_kind_keeps_its_slot() {
    let mut bank = Bank::default();
    assert!(bank.is_empty());
    assert_eq!(bank.put(Item::CopperOre, 5), 0);
    assert_eq!(bank.put(Item::HealingPotion, 2), 0);
    assert_eq!(bank.put(Item::BronzeSword, 1), 0);
    // More of each goes onto its stack, gear and potions too.
    assert_eq!((bank.put(Item::HealingPotion, 3), bank.put(Item::BronzeSword, 1)), (0, 0));
    assert_eq!((bank.count(Item::CopperOre), bank.count(Item::HealingPotion), bank.count(Item::BronzeSword)), (5, 5, 2));
    assert_eq!(bank.kinds(), 3);
    assert_eq!(bank.slots()[1], Some(Stack { item: Item::HealingPotion, count: 5 }));
    // Taking some leaves the rest where it was; taking all frees the slot
    // and moves nothing else.
    assert_eq!(bank.take(0, 2), 2);
    assert_eq!(bank.take(1, 99), 5);
    assert_eq!((bank.slots()[0], bank.slots()[1], bank.slots()[2]), (Some(Stack { item: Item::CopperOre, count: 3 }), None, Some(Stack { item: Item::BronzeSword, count: 2 })));
    assert_eq!((bank.take(1, 1), bank.take(77, 1)), (0, 0), "nothing there to take");
    // A new kind goes into the first free slot.
    bank.put(Item::TinOre, 1);
    assert_eq!(bank.slots()[1], Some(Stack { item: Item::TinOre, count: 1 }));
}

#[test]
fn a_new_kind_does_not_go_into_a_bank_with_every_slot_taken() {
    let mut bank = Bank::default();
    for slot in 0..SLOTS {
        bank.set_slot(slot, Some(Stack { item: Item::CopperOre, count: 1 }));
    }
    assert_eq!(bank.kinds(), SLOTS);
    assert_eq!(bank.put(Item::TinOre, 4), 4, "none of it went in");
    assert_eq!(bank.put(Item::CopperOre, 4), 0, "more of what is there still does");
    // And never more than a slot can count.
    bank.set_slot(0, Some(Stack { item: Item::CopperOre, count: u32::MAX - 1 }));
    assert_eq!(bank.put(Item::CopperOre, 5), 4);
}

#[test]
fn walking_up_to_the_chest_is_being_at_the_bank_and_walking_off_is_leaving_it() {
    let mut sim = by_the_chest();
    assert!(sim.bank_at((5, 2)) && !sim.bank_at((4, 2)) && !sim.banking());
    sim.click((5, 2));
    assert_eq!(sim.using(), Some((5, 2)), "on the way to it");
    for _ in 0..20 {
        sim.tick();
    }
    assert!(sim.banking());
    assert_eq!((sim.player.tile, sim.using()), ((4, 2), Some((5, 2))), "beside it, and still set on it");
    assert!(!sim.player.busy(), "standing at the bank is standing still");
    sim.click((1, 1));
    assert!(!sim.banking() && sim.using().is_none());
    // Stepping back without walking anywhere does it too.
    let mut sim = banking();
    sim.leave_bank();
    assert!(!sim.banking());
}

#[test]
fn things_go_in_from_the_slot_that_was_pressed_and_potions_stack_once_inside() {
    let mut sim = banking();
    sim.inventory.add(Item::CopperOre, 12);
    sim.inventory.add(Item::HealingPotion, 3);
    assert_eq!(sim.inventory.slots()[2], Some(Stack { item: Item::HealingPotion, count: 1 }));
    // One ore, then five more.
    assert_eq!((sim.deposit(0, 1), sim.deposit(0, 5)), (1, 5));
    assert_eq!((sim.inventory.count(Item::CopperOre), sim.bank.count(Item::CopperOre)), (6, 6));
    // The middle potion: that slot empties, the ones either side stay.
    assert_eq!(sim.deposit(2, 1), 1);
    assert_eq!((sim.inventory.slots()[1].map(|s| s.item), sim.inventory.slots()[2], sim.inventory.slots()[3].map(|s| s.item)), (Some(Item::HealingPotion), None, Some(Item::HealingPotion)));
    // Asked for more than the pressed slot holds, the rest come from the others.
    assert_eq!(sim.deposit(1, 5), 2);
    assert_eq!((sim.inventory.count(Item::HealingPotion), sim.bank.count(Item::HealingPotion)), (0, 3));
    assert_eq!(sim.bank.kinds(), 2, "three potions are one slot of the bank");
    // More than there is moves what there is; an empty slot moves nothing.
    assert_eq!((sim.deposit(0, 100), sim.deposit(0, 1), sim.deposit(99, 1)), (6, 0, 0));
    assert!(sim.inventory.is_empty() && sim.take_events().is_empty());
}

#[test]
fn things_come_out_as_far_as_there_is_room_in_the_bag() {
    let mut sim = banking();
    sim.bank.put(Item::CopperOre, 40);
    sim.bank.put(Item::HealingPotion, 40);
    assert_eq!(sim.withdraw(0, 1), 1);
    assert_eq!(sim.withdraw(0, 10), 10);
    assert_eq!((sim.inventory.count(Item::CopperOre), sim.bank.count(Item::CopperOre)), (11, 29));
    // Potions take a slot each in the bag: 29 are left free.
    assert_eq!(sim.withdraw(1, u32::MAX), 29);
    assert_eq!((sim.inventory.count(Item::HealingPotion), sim.bank.count(Item::HealingPotion)), (29, 11));
    assert_eq!(sim.take_events(), vec![Event::InventoryFull]);
    // Ore still stacks onto what is carried.
    assert_eq!(sim.withdraw(0, u32::MAX), 29);
    assert_eq!(sim.bank.slots()[0], None);
    assert_eq!(sim.withdraw(0, 1), 0, "an empty slot gives nothing");
}

#[test]
fn nothing_moves_away_from_the_bank() {
    let mut sim = by_the_chest();
    sim.inventory.add(Item::CopperOre, 3);
    sim.bank.put(Item::TinOre, 3);
    assert_eq!((sim.deposit(0, 1), sim.withdraw(0, 1)), (0, 0));
    sim.deposit_all();
    assert_eq!((sim.inventory.count(Item::CopperOre), sim.bank.count(Item::TinOre), sim.bank.count(Item::CopperOre)), (3, 3, 0));
}

#[test]
fn the_whole_bag_goes_in_at_once_and_a_full_bank_says_so_once() {
    let mut sim = banking();
    sim.inventory.add(Item::CopperOre, 9);
    sim.inventory.add(Item::BronzeSword, 2);
    sim.inventory.add(Item::HealingPotion, 4);
    sim.deposit_all();
    assert!(sim.inventory.is_empty() && sim.take_events().is_empty());
    assert_eq!((sim.bank.count(Item::CopperOre), sim.bank.count(Item::BronzeSword), sim.bank.count(Item::HealingPotion)), (9, 2, 4));
    // Every other slot taken: nothing new fits, and that is said once.
    for slot in 3..SLOTS {
        sim.bank.set_slot(slot, Some(Stack { item: Item::CopperOre, count: 1 }));
    }
    sim.inventory.add(Item::TinOre, 2);
    sim.inventory.add(Item::GoblinTooth, 2);
    sim.inventory.add(Item::HealingPotion, 1);
    sim.deposit_all();
    assert_eq!(sim.take_events(), vec![Event::BankFull]);
    assert_eq!((sim.inventory.count(Item::TinOre), sim.inventory.count(Item::GoblinTooth), sim.inventory.count(Item::HealingPotion)), (2, 2, 0), "what there was a stack for went in");
    assert_eq!(sim.deposit(0, 1), 0);
    assert_eq!(sim.take_events(), vec![Event::BankFull]);
}

#[test]
fn the_bank_is_saved_and_dying_does_not_touch_it() {
    let mut sim = banking();
    sim.inventory.add(Item::CopperOre, 5);
    sim.deposit(0, 5);
    sim.bank.put(Item::BronzeSword, 3);
    sim.bank.take(0, 5);
    // The sword is in the second slot, the first is free: slots are kept.
    let who = Character { name: "Alva".to_owned(), played: 7 };
    let saved = sim.snapshot(&who);
    let back = Saved::decode(&saved.encode()).expect("decodes");
    assert_eq!(back, saved);
    assert_eq!((back.bank.slots()[0], back.bank.slots()[1]), (None, Some(Stack { item: Item::BronzeSword, count: 3 })));
    let world = std::mem::take(&mut sim.world);
    let resumed = Sim::resume(world, &back);
    assert_eq!(resumed.bank, sim.bank);
    assert!(!resumed.banking(), "they come back standing by it, not at it");
    // An empty bank is no section at all, as in a save from before banks.
    let fresh = Sim::new(World::new()).snapshot(&who);
    assert!(!fresh.encode().windows(4).any(|w| w == b"BANK"));
    assert_eq!(Saved::decode(&fresh.encode()).expect("decodes").bank, Bank::default());
}

#[test]
fn a_bank_with_one_kind_in_two_slots_is_refused() {
    let mut sim = banking();
    sim.bank.set_slot(0, Some(Stack { item: Item::TinOre, count: 2 }));
    sim.bank.set_slot(5, Some(Stack { item: Item::TinOre, count: 1 }));
    let bytes = sim.snapshot(&Character::default()).encode();
    let why = Saved::decode(&bytes).expect_err("refused");
    assert!(why.contains("Tin ore in two slots"), "{why}");
}

#[test]
fn what_is_carried_goes_to_the_grave_and_what_is_banked_stays_banked() {
    let mut sim = banking();
    sim.inventory.add(Item::CopperOre, 4);
    sim.deposit(0, 3);
    let before = sim.bank.clone();
    sim.player.health = 0;
    // Killed where they stand, as a monster's blow would do it.
    sim.grave = Some(crate::combat::Grave { tile: sim.player.tile, items: std::mem::take(&mut sim.inventory) });
    sim.stop();
    assert_eq!(sim.bank, before);
    assert!(!sim.banking(), "and they are no longer at the bank");
    assert_eq!(sim.inventory, Inventory::default());
}
