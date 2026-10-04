//! Tests for smelting and forging.

use super::*;
use crate::skills::xp_for_level;
use crate::terrain::Terrain;
use crate::world::World;

/// A small field with a furnace at (5, 2) and an anvil at (8, 2), the
/// player standing at (3, 2).
fn smithy() -> Sim {
    let mut world = World::new();
    for y in 0..6 {
        for x in 0..12 {
            world.set_terrain(x, y, Terrain::Grass);
        }
    }
    world.place_prop(PropKind::Furnace, 5, 2);
    world.place_prop(PropKind::Anvil, 8, 2);
    let mut sim = Sim::new(world);
    sim.player.tile = (3, 2);
    sim.player.from = (3, 2);
    sim
}

/// Tick until the player is idle, gathering what happened.
fn run(sim: &mut Sim) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..2000 {
        sim.tick();
        events.extend(sim.take_events());
        if !sim.player.busy() {
            break;
        }
    }
    events
}

fn recipe_for(item: Item) -> usize {
    RECIPES.iter().position(|r| r.makes == item).expect("a recipe")
}

#[test]
fn every_recipe_is_sound_and_the_set_can_be_made_in_order_without_filler() {
    for (i, r) in RECIPES.iter().enumerate() {
        assert!(!r.from.is_empty() && r.from.iter().all(|&(_, n)| n > 0) && r.ticks > 0 && r.xp > 0 && r.level >= 1);
        assert!(RECIPES[..i].iter().all(|other| other.makes != r.makes), "one way to make each thing");
        assert!(!r.from.iter().any(|&(item, _)| item == r.makes));
    }
    for station in Station::ALL {
        let levels: Vec<u8> = RECIPES.iter().filter(|r| r.station == station).map(|r| r.level).collect();
        assert!(!levels.is_empty() && levels.is_sorted(), "{station:?} lists its easiest first: {levels:?}");
    }
    // Smelting the bars for a thing and then forging it, each in turn,
    // is always enough experience for the next.
    let mut skills = crate::skills::Skills::default();
    let bar = RECIPES[recipe_for(Item::BronzeBar)];
    for r in RECIPES.iter().filter(|r| r.station == Station::Anvil) {
        skills.gain(Skill::Smithing, bar.xp * r.from[0].1);
        assert!(skills.level(Skill::Smithing) >= r.level, "{:?} needs {} at level {}", r.makes, r.level, skills.level(Skill::Smithing));
        skills.gain(Skill::Smithing, r.xp);
    }
}

#[test]
fn stations_are_in_the_way_and_only_they_are_stations() {
    let sim = smithy();
    assert_eq!((sim.station_at((5, 2)), sim.station_at((8, 2)), sim.station_at((3, 2))), (Some(Station::Furnace), Some(Station::Anvil), None));
    assert!(!sim.world.walkable(5, 2) && !sim.world.walkable(8, 2));
    assert_eq!(PropKind::Rock.station(), None);
    assert_eq!(PropKind::Furnace.node(), None);
}

#[test]
fn clicking_a_station_walks_up_to_it_and_walking_off_leaves_it() {
    let mut sim = smithy();
    assert_eq!(sim.station(), None);
    sim.click((5, 2));
    let events = run(&mut sim);
    assert_eq!(events, vec![Event::AtStation { station: Station::Furnace }]);
    assert_eq!(sim.station(), Some(Station::Furnace));
    assert_eq!(sim.player.tile, (4, 2));
    // Clicking it again from beside it says so again.
    sim.click((5, 2));
    assert_eq!(run(&mut sim), vec![Event::AtStation { station: Station::Furnace }]);
    sim.walk_to((2, 2));
    assert_eq!(sim.station(), None, "the moment they turn to go");
}

#[test]
fn smelting_takes_one_of_each_ore_a_bar_and_its_ticks() {
    let mut sim = smithy();
    sim.inventory.add(Item::CopperOre, 3);
    sim.inventory.add(Item::TinOre, 2);
    let bar = recipe_for(Item::BronzeBar);
    assert_eq!(sim.makeable(&RECIPES[bar]), 2, "as many as the scarcer ore");
    // Not from where they stand: only at the furnace.
    sim.make(bar, 5);
    assert_eq!(sim.making(), None);
    sim.click((5, 2));
    run(&mut sim);
    sim.make(bar, 5);
    assert_eq!(sim.making().map(|(r, left, _)| (r.makes, left)), Some((Item::BronzeBar, 2)), "asked for five, set to the two there is ore for");
    let mut made_on = Vec::new();
    for tick in 1..=10 {
        let due = sim.swing_due();
        sim.tick();
        let got = sim.take_events().contains(&Event::Got { item: Item::BronzeBar, count: 1 });
        assert_eq!((due, sim.player.swung), (got, got), "tick {tick}");
        if got {
            made_on.push(tick);
        }
    }
    assert_eq!(made_on, vec![RECIPES[bar].ticks, RECIPES[bar].ticks * 2]);
    assert_eq!((sim.inventory.count(Item::BronzeBar), sim.inventory.count(Item::CopperOre), sim.inventory.count(Item::TinOre)), (2, 1, 0));
    assert_eq!(sim.skills.xp(Skill::Smithing), RECIPES[bar].xp * 2);
    assert!(!sim.player.busy());
    assert_eq!(sim.station(), Some(Station::Furnace), "still at it, to make more");
}

#[test]
fn forging_wants_the_anvil_the_level_and_the_bars() {
    let mut sim = smithy();
    sim.inventory.add(Item::BronzeBar, 7);
    let (pickaxe, sword, chest) = (recipe_for(Item::BronzePickaxe), recipe_for(Item::BronzeSword), recipe_for(Item::BronzeChestplate));
    // The furnace does not forge.
    sim.click((5, 2));
    run(&mut sim);
    sim.make(pickaxe, 1);
    assert_eq!(sim.making(), None);
    sim.click((8, 2));
    assert_eq!(run(&mut sim), vec![Event::AtStation { station: Station::Anvil }]);
    // Too hard for a beginner: said, and nothing made.
    assert!(!sim.can_make(&RECIPES[sword]));
    sim.make(sword, 1);
    assert_eq!(sim.take_events(), vec![Event::TooHard { skill: Skill::Smithing, level: RECIPES[sword].level }]);
    assert_eq!(sim.making(), None);
    // A pickaxe is not.
    sim.make(pickaxe, 1);
    let events = run(&mut sim);
    assert!(events.contains(&Event::Got { item: Item::BronzePickaxe, count: 1 }));
    assert!(events.contains(&Event::LevelUp { skill: Skill::Smithing, level: 2 }), "{events:?}");
    assert_eq!((sim.inventory.count(Item::BronzePickaxe), sim.inventory.count(Item::BronzeBar)), (1, 5));
    // Gear does not stack: two swords are two slots.
    sim.skills.set_xp(Skill::Smithing, xp_for_level(5));
    sim.make(sword, 99);
    run(&mut sim);
    assert_eq!((sim.inventory.count(Item::BronzeSword), sim.inventory.count(Item::BronzeBar)), (2, 1));
    assert_eq!(sim.inventory.slots().iter().flatten().filter(|s| s.item == Item::BronzeSword).count(), 2);
    // One bar is not a chestplate.
    assert_eq!(sim.makeable(&RECIPES[chest]), 0);
    sim.make(chest, 1);
    assert_eq!(sim.making(), None);
}

#[test]
fn asking_for_a_number_makes_that_many_and_walking_off_stops_it() {
    let mut sim = smithy();
    sim.inventory.add(Item::CopperOre, 10);
    sim.inventory.add(Item::TinOre, 10);
    sim.click((5, 2));
    run(&mut sim);
    let bar = recipe_for(Item::BronzeBar);
    sim.make(bar, 1);
    run(&mut sim);
    assert_eq!(sim.inventory.count(Item::BronzeBar), 1);
    sim.make(bar, 5);
    run(&mut sim);
    assert_eq!(sim.inventory.count(Item::BronzeBar), 6);
    // Walking off part way through one leaves it unmade and the ore kept.
    sim.make(bar, 4);
    sim.tick();
    sim.walk_to((2, 2));
    run(&mut sim);
    assert_eq!((sim.inventory.count(Item::BronzeBar), sim.inventory.count(Item::CopperOre)), (6, 4));
    assert_eq!(sim.making(), None);
}

#[test]
fn with_no_room_for_what_is_made_nothing_is_taken() {
    let mut sim = smithy();
    sim.skills.set_xp(Skill::Smithing, xp_for_level(5));
    // A full bag: bars in one slot, teeth in every other, each its own.
    sim.inventory.add(Item::BronzeBar, 3);
    for slot in 1..crate::items::SLOTS {
        sim.inventory.set_slot(slot, Some(crate::items::Stack { item: Item::GoblinTooth, count: 1 }));
    }
    sim.click((8, 2));
    run(&mut sim);
    // Two of three bars leave a bar in the slot: no room for a sword.
    sim.make(recipe_for(Item::BronzeSword), 1);
    let events = run(&mut sim);
    assert_eq!(events, vec![Event::InventoryFull]);
    assert_eq!((sim.inventory.count(Item::BronzeBar), sim.inventory.count(Item::BronzeSword)), (3, 0));
    // All three bars empty the slot: the shield goes where they were.
    sim.make(recipe_for(Item::BronzeShield), 1);
    run(&mut sim);
    assert_eq!((sim.inventory.count(Item::BronzeBar), sim.inventory.count(Item::BronzeShield)), (0, 1));
}

#[test]
fn a_tin_rock_gives_tin_and_a_pickaxe_in_the_bag_makes_every_swing_quicker() {
    let tin = PropKind::TinRock.node().expect("a node");
    assert_eq!((tin.item, tin.skill, tin.level), (Item::TinOre, Skill::Mining, 1));
    let mut sim = smithy();
    assert_eq!(sim.swing_ticks(&tin), 3);
    sim.inventory.add(Item::BronzeSword, 1);
    assert_eq!(sim.swing_ticks(&tin), 3, "a sword is no pickaxe");
    sim.inventory.add(Item::BronzePickaxe, 1);
    assert_eq!(sim.swing_ticks(&tin), 2);
    sim.inventory.add(Item::BronzePickaxe, 1);
    assert_eq!(sim.swing_ticks(&tin), 2, "two are no quicker than one");

    // Mined out, the swings come every two ticks.
    sim.world.place_prop(PropKind::TinRock, 3, 4);
    sim.click((3, 4));
    let mut swings = Vec::new();
    for tick in 0..200 {
        sim.tick();
        if sim.take_events().iter().any(|e| matches!(e, Event::Swing { .. })) {
            swings.push(tick);
        }
        if !sim.player.busy() {
            break;
        }
    }
    assert_eq!(sim.inventory.count(Item::TinOre), 3);
    assert!(swings.windows(2).all(|w| w[1] - w[0] == 2), "{swings:?}");
}

#[test]
fn a_station_edited_away_mid_make_drops_the_work() {
    let mut sim = smithy();
    sim.inventory.add(Item::CopperOre, 2);
    sim.inventory.add(Item::TinOre, 2);
    sim.click((5, 2));
    run(&mut sim);
    sim.make(recipe_for(Item::BronzeBar), 2);
    sim.tick();
    sim.world.remove_prop(5, 2);
    run(&mut sim);
    assert_eq!((sim.making(), sim.station(), sim.inventory.count(Item::BronzeBar)), (None, None, 0));
}
