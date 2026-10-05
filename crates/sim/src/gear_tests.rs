//! Tests for what is worn.

use super::*;
use crate::combat::hit_chance;
use crate::monsters::MonsterKind;
use crate::save::{Character, Saved};
use crate::terrain::Terrain;
use crate::world::{Marker, World};

fn field() -> World {
    let mut world = World::new();
    for y in 0..12 {
        for x in 0..30 {
            world.set_terrain(x, y, Terrain::Grass);
        }
    }
    world.set_marker(Marker::PlayerStart, 2, 6);
    world.set_spawn(22, 6, Some(MonsterKind::Goblin));
    world
}

const SET: [Item; 7] = [Item::BronzeSword, Item::BronzeShield, Item::BronzeHelmet, Item::BronzeChestplate, Item::BronzeGreaves, Item::BronzeGauntlets, Item::BronzeBoots];

/// A player with the whole bronze set on.
fn armoured() -> Sim {
    let mut sim = Sim::new(field());
    for item in SET {
        sim.inventory.add(item, 1);
        sim.equip(0);
    }
    sim
}

#[test]
fn every_slot_has_its_names_and_the_set_fills_every_slot_once() {
    for (i, slot) in Slot::ALL.iter().enumerate() {
        assert_eq!(Slot::from_name(slot.name()), Some(*slot));
        assert!(Slot::ALL[..i].iter().all(|other| other.name() != slot.name() && other.label() != slot.label()));
    }
    let slots: Vec<Slot> = SET.iter().map(|item| item.slot().expect("worn")).collect();
    assert_eq!(slots, Slot::ALL.to_vec());
    for item in Item::ALL {
        assert_eq!(item.slot().is_some(), item.metal().is_some());
        if item.slot().is_none() {
            assert_eq!(item.bonus(), Bonus::default(), "{item:?} is not worn and adds nothing");
        } else {
            assert!(!item.stacks(), "gear is one to a slot");
        }
    }
    assert_eq!((Item::BronzePickaxe.slot(), Item::BronzeBar.slot()), (None, None), "a pickaxe works from the bag");
}

#[test]
fn gear_goes_on_from_the_bag_and_trades_places_with_what_was_worn() {
    let mut sim = Sim::new(field());
    sim.inventory.add(Item::CopperOre, 3);
    sim.inventory.add(Item::BronzeSword, 2);
    // Ore cannot be worn; an empty slot and one that is not there hold nothing.
    for slot in [0, 5, 99] {
        sim.equip(slot);
    }
    assert_eq!(sim.gear, Gear::default());
    sim.equip(1);
    assert_eq!(sim.gear.worn(Slot::Weapon), Some(Item::BronzeSword));
    assert_eq!(sim.inventory.slots()[1], None, "it left the bag");
    // The other sword trades places with it: the bag slot is not lost.
    sim.equip(2);
    assert_eq!((sim.gear.worn(Slot::Weapon), sim.inventory.count(Item::BronzeSword)), (Some(Item::BronzeSword), 1));
    assert!(sim.inventory.slots()[2].is_some());
    assert_eq!(sim.gear.iter().count(), 1);
}

#[test]
fn gear_comes_off_into_the_bag_when_there_is_room() {
    let mut sim = armoured();
    assert!(sim.inventory.is_empty());
    sim.unequip(Slot::Head);
    assert_eq!((sim.gear.worn(Slot::Head), sim.inventory.count(Item::BronzeHelmet)), (None, 1));
    sim.unequip(Slot::Head);
    assert_eq!(sim.inventory.count(Item::BronzeHelmet), 1, "nothing there to take off twice");
    // A full bag keeps it on, and says so.
    for slot in 0..crate::items::SLOTS {
        sim.inventory.set_slot(slot, Some(Stack { item: Item::BronzeBoots, count: 1 }));
    }
    sim.take_events();
    sim.unequip(Slot::Body);
    assert_eq!(sim.gear.worn(Slot::Body), Some(Item::BronzeChestplate));
    assert_eq!(sim.take_events(), vec![Event::InventoryFull]);
}

#[test]
fn a_sword_makes_blows_surer_and_harder_and_armour_makes_the_player_harder_to_hit() {
    let bare = Sim::new(field());
    let geared = armoured();
    assert_eq!(geared.gear.bonus(), Bonus { accuracy: 2, damage: 2, armour: 10 });
    assert_eq!((bare.max_hit(), geared.max_hit()), (2, 4));
    assert_eq!((bare.accuracy(), geared.accuracy()), (1, 3));
    assert_eq!((bare.armour(), geared.armour()), (1, 11));
    let goblin = MonsterKind::Goblin.stats();
    assert!(hit_chance(geared.accuracy(), goblin.defence) > hit_chance(bare.accuracy(), goblin.defence));
    assert!((hit_chance(goblin.attack, geared.armour()) - 0.20).abs() < 1e-9, "a goblin lands one blow in five on full bronze");

    // Played out: over many fights the armoured player is hurt far less
    // and wins far quicker.
    let fight = |geared: bool, seed: u64| {
        let mut sim = if geared { armoured() } else { Sim::new(field()) };
        sim.reseed(seed);
        sim.player.tile = (21, 6);
        sim.player.from = sim.player.tile;
        sim.attack(0);
        let (mut ticks, mut lost) = (0u32, 0u32);
        while sim.monsters[0].alive() && ticks < 500 {
            let before = sim.player.health;
            sim.tick();
            lost += before.saturating_sub(sim.player.health);
            ticks += 1;
        }
        (ticks, lost)
    };
    let total = |geared: bool| (0..200).map(|seed| fight(geared, seed)).fold((0, 0), |sum, (t, l)| (sum.0 + t, sum.1 + l));
    let (bare, geared) = (total(false), total(true));
    assert!(geared.0 * 3 < bare.0 * 2, "a third quicker at least: {geared:?} against {bare:?}");
    assert!(geared.1 * 3 < bare.1, "and far less hurt: {geared:?} against {bare:?}");
}

#[test]
fn what_is_worn_stays_on_through_death_while_the_bag_stays_in_the_grave() {
    let mut sim = armoured();
    sim.inventory.add(Item::CopperOre, 4);
    sim.player.tile = (21, 6);
    sim.player.from = sim.player.tile;
    for _ in 0..3000 {
        sim.player.health = sim.player.health.min(1);
        sim.tick_world();
        if sim.take_events().contains(&Event::Died) {
            break;
        }
    }
    assert!(sim.grave.as_ref().is_some_and(|g| g.items.count(Item::CopperOre) == 4));
    assert!(sim.inventory.is_empty());
    assert_eq!(sim.gear.iter().count(), 7, "every piece still on");
}

#[test]
fn gear_is_saved_and_a_save_from_before_gear_reads_as_wearing_nothing() {
    let mut sim = armoured();
    sim.unequip(Slot::Shield);
    let saved = sim.snapshot(&Character { name: "Alva".to_owned(), played: 5 });
    let read = Saved::decode(&saved.encode()).expect("it reads back");
    assert_eq!(read, saved);
    let back = Sim::resume(field(), &read);
    assert_eq!(back.gear, sim.gear);
    assert_eq!((back.gear.worn(Slot::Shield), back.inventory.count(Item::BronzeShield)), (None, 1));
    assert_eq!(back.max_hit(), 4, "and it counts from the first tick");

    // Wearing nothing writes no gear section at all, as saves from
    // before there was gear have none.
    let bare = Sim::new(field()).snapshot(&Character::default());
    assert!(!bare.encode().windows(4).any(|w| w == b"GEAR"));
    assert_eq!(Saved::decode(&bare.encode()).unwrap().gear, Gear::default());
}

#[test]
fn a_save_with_gear_in_the_wrong_slot_is_refused() {
    let mut sim = Sim::new(field());
    sim.inventory.add(Item::BronzeSword, 1);
    sim.equip(0);
    let bytes = sim.snapshot(&Character::default()).encode();
    // Call the weapon slot the shield slot: same length, so nothing else moves.
    let at = bytes.windows(6).position(|w| w == b"weapon").expect("the slot's name");
    let mut wrong = bytes.clone();
    wrong[at..at + 6].copy_from_slice(b"shield");
    let error = Saved::decode(&wrong).expect_err("a sword is not a shield");
    assert!(error.contains("Bronze sword"), "{error}");
}

#[test]
fn gear_taken_off_goes_into_the_slot_it_was_dropped_on_or_trades_with_gear_of_its_kind() {
    let mut sim = Sim::new(crate::world::World::new());
    sim.inventory.add(Item::BronzeSword, 1);
    sim.inventory.add(Item::CopperOre, 3);
    sim.equip(0);
    assert_eq!(sim.gear.worn(Slot::Weapon), Some(Item::BronzeSword));
    // Onto an empty slot: there.
    sim.unequip_into(Slot::Weapon, 9);
    assert_eq!((sim.gear.worn(Slot::Weapon), sim.inventory.slots()[9].map(|s| s.item)), (None, Some(Item::BronzeSword)));
    // Onto another weapon: the two trade places.
    sim.equip(9);
    sim.inventory.set_slot(4, Some(Stack { item: Item::BronzeSword, count: 1 }));
    sim.unequip_into(Slot::Weapon, 4);
    assert_eq!((sim.gear.worn(Slot::Weapon), sim.inventory.slots()[4].map(|s| s.item)), (Some(Item::BronzeSword), Some(Item::BronzeSword)));
    // Onto ore, or off the end of the bag: wherever there is room.
    sim.inventory.set_slot(4, None);
    sim.unequip_into(Slot::Weapon, 1);
    assert_eq!((sim.gear.worn(Slot::Weapon), sim.inventory.slots()[1].map(|s| s.item), sim.inventory.count(Item::BronzeSword)), (None, Some(Item::CopperOre), 1));
    // Nothing worn there: nothing happens.
    sim.unequip_into(Slot::Head, 20);
    assert_eq!(sim.inventory.slots()[20], None);
}
