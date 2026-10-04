//! Tests for herbs, brewing and what potions do.

use super::*;
use crate::items::{SLOTS, Stack};
use crate::making::{RECIPES, Station};
use crate::monsters::MonsterKind;
use crate::skills::{Skill, xp_for_level};
use crate::terrain::Terrain;
use crate::world::{PropKind, World};

/// A field with a purple herb at (5, 2), a cyan one at (7, 2) and a
/// cauldron at (9, 2); the player at (3, 2).
fn garden() -> Sim {
    let mut world = World::new();
    for y in 0..6 {
        for x in 0..14 {
            world.set_terrain(x, y, Terrain::Grass);
        }
    }
    world.place_prop(PropKind::HerbPurple, 5, 2);
    world.place_prop(PropKind::HerbCyan, 7, 2);
    world.place_prop(PropKind::Cauldron, 9, 2);
    let mut sim = Sim::new(world);
    sim.player.tile = (3, 2);
    sim.player.from = (3, 2);
    sim
}

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

/// Put one of an item in the bag and say which slot it went in.
fn carry(sim: &mut Sim, item: Item) -> usize {
    assert_eq!(sim.inventory.add(item, 1), 0);
    sim.inventory.slots().iter().rposition(|s| s.is_some_and(|s| s.item == item)).unwrap()
}

#[test]
fn a_herb_is_picked_from_on_top_of_it_never_fails_and_grows_back() {
    for seed in 0..30 {
        let mut sim = garden();
        sim.reseed(seed);
        sim.click((5, 2));
        let events = run(&mut sim);
        assert_eq!(sim.player.tile, (5, 2), "a herb is not in the way: they walk onto it");
        assert_eq!(sim.inventory.count(Item::PurpleHerb), 2, "seed {seed}");
        assert!(!events.iter().any(|e| matches!(e, Event::Swing { hit: false, .. })), "seed {seed}: a pick never fails");
        assert_eq!(sim.skills.xp(Skill::Herbalism), 20);
        assert!(sim.is_spent((5, 2)));
    }
    let mut sim = garden();
    sim.click((5, 2));
    run(&mut sim);
    // Picked bare, there is nothing to pick until it grows back.
    sim.click((5, 2));
    assert!(run(&mut sim).contains(&Event::Spent { at: (5, 2) }));
    for _ in 0..80 {
        sim.tick();
    }
    assert_eq!(sim.node_left((5, 2)), Some(2));
    assert!(sim.world.walkable(5, 2), "and it was never in the way");
}

#[test]
fn the_cyan_herb_wants_more_herbalism() {
    let mut sim = garden();
    sim.click((7, 2));
    assert!(run(&mut sim).contains(&Event::TooHard { skill: Skill::Herbalism, level: 3 }));
    assert_eq!(sim.inventory.count(Item::CyanHerb), 0);
    sim.skills.set_xp(Skill::Herbalism, xp_for_level(3));
    sim.click((7, 2));
    run(&mut sim);
    assert_eq!(sim.inventory.count(Item::CyanHerb), 2);
}

#[test]
fn potions_are_brewed_at_the_cauldron_with_alchemy_and_each_takes_a_slot() {
    let mut sim = garden();
    sim.inventory.add(Item::PurpleHerb, 3);
    sim.inventory.add(Item::CyanHerb, 3);
    sim.inventory.add(Item::GoblinTooth, 4);
    assert_eq!((sim.station_at((9, 2)), Station::Cauldron.skill()), (Some(Station::Cauldron), Skill::Alchemy));
    sim.click((9, 2));
    assert_eq!(run(&mut sim), vec![Event::AtStation { station: Station::Cauldron }]);
    let brew = |item: Item| RECIPES.iter().position(|r| r.makes == item).unwrap();
    for potion in Potion::ALL {
        assert_eq!(RECIPES[brew(potion.item())].station, Station::Cauldron);
        assert_eq!(potion.item().potion(), Some(potion));
        assert!(!potion.item().stacks());
    }
    // A beginner brews healing potions, and nothing harder.
    sim.make(brew(Item::StrengthPotion), 1);
    assert_eq!(sim.take_events(), vec![Event::TooHard { skill: Skill::Alchemy, level: 2 }]);
    sim.make(brew(Item::HealingPotion), 5);
    run(&mut sim);
    assert_eq!(sim.inventory.count(Item::HealingPotion), 3, "as many as there were purple herbs");
    assert_eq!(sim.inventory.slots().iter().flatten().filter(|s| s.item == Item::HealingPotion).count(), 3, "one to a slot");
    assert_eq!(sim.skills.xp(Skill::Alchemy), 60);
    assert_eq!(sim.skills.xp(Skill::Smithing), 0, "brewing is not smithing");
    // Sixty experience is level 2: strength potions now.
    assert!(sim.skills.level(Skill::Alchemy) >= 2);
    sim.make(brew(Item::StrengthPotion), 1);
    run(&mut sim);
    assert_eq!((sim.inventory.count(Item::StrengthPotion), sim.inventory.count(Item::GoblinTooth), sim.inventory.count(Item::CyanHerb)), (1, 0, 2));
}

#[test]
fn a_healing_potion_gives_back_health_at_once_and_is_not_wasted_on_the_whole() {
    let mut sim = garden();
    let slot = carry(&mut sim, Item::HealingPotion);
    // Whole: it stays in the bag.
    sim.drink(slot);
    assert_eq!(sim.inventory.count(Item::HealingPotion), 1);
    assert!(sim.take_events().is_empty());
    sim.player.health = 2;
    sim.drink(slot);
    assert_eq!((sim.player.health, sim.inventory.count(Item::HealingPotion)), (2 + HEALING, 0));
    assert_eq!(sim.take_events(), vec![Event::Drank { potion: Potion::Healing, healed: HEALING }]);
    // Nearly whole: only what was missing.
    sim.tonics = Tonics::default();
    let slot = carry(&mut sim, Item::HealingPotion);
    sim.player.health = sim.max_health() - 4;
    sim.drink(slot);
    assert_eq!(sim.player.health, sim.max_health());
    assert_eq!(sim.take_events(), vec![Event::Drank { potion: Potion::Healing, healed: 4 }]);
}

#[test]
fn after_one_the_next_of_its_kind_waits_and_other_kinds_do_not() {
    let mut sim = garden();
    let (first, second, other) = (carry(&mut sim, Item::HealingPotion), carry(&mut sim, Item::HealingPotion), carry(&mut sim, Item::StrengthPotion));
    sim.player.health = 1;
    sim.drink(first);
    assert_eq!(sim.drink_wait(Potion::Healing), DRINK_TICKS);
    sim.drink(second);
    assert_eq!(sim.inventory.count(Item::HealingPotion), 1, "not yet");
    sim.drink(other);
    assert_eq!(sim.inventory.count(Item::StrengthPotion), 0, "a different kind does not wait");
    for left in (0..DRINK_TICKS).rev() {
        assert!(sim.waiting(), "the world ticks on while a wait runs");
        sim.player.health = 1;
        sim.tick_world();
        assert_eq!(sim.drink_wait(Potion::Healing), left);
    }
    sim.drink(second);
    assert_eq!(sim.inventory.count(Item::HealingPotion), 0);
}

#[test]
fn strength_and_stoneskin_last_their_time_and_then_wear_off() {
    let mut sim = garden();
    let (hit, armour) = (sim.max_hit(), sim.armour());
    let strength = carry(&mut sim, Item::StrengthPotion);
    sim.drink(strength);
    let stoneskin = carry(&mut sim, Item::StoneskinPotion);
    sim.drink(stoneskin);
    assert_eq!(sim.take_events(), vec![Event::Drank { potion: Potion::Strength, healed: 0 }, Event::Drank { potion: Potion::Stoneskin, healed: 0 }]);
    assert_eq!((sim.max_hit(), sim.armour()), (hit + STRENGTH, armour + STONESKIN));
    assert_eq!(sim.potion_left(Potion::Strength), LASTING_TICKS);
    for _ in 0..LASTING_TICKS - 1 {
        sim.tick_world();
    }
    assert_eq!((sim.max_hit(), sim.armour(), sim.potion_left(Potion::Stoneskin)), (hit + STRENGTH, armour + STONESKIN, 1));
    sim.tick_world();
    assert_eq!((sim.max_hit(), sim.armour()), (hit, armour));
    assert!(!sim.waiting(), "nothing left running");
}

#[test]
fn regeneration_gives_health_back_steadily_even_in_a_fight() {
    let mut sim = garden();
    sim.skills.set_xp(Skill::Vitality, xp_for_level(20));
    sim.world.set_spawn(4, 2, Some(MonsterKind::Goblin));
    sim.settle_player();
    sim.player.health = 10;
    let slot = carry(&mut sim, Item::RegenerationPotion);
    sim.drink(slot);
    // Fighting all the while, so the quick mending out of a fight never comes.
    let mut gained = 0;
    for _ in 0..REGENERATION_TICKS {
        sim.last_fight = Some(sim.ticks);
        sim.monsters.clear();
        let before = sim.player.health;
        sim.tick_world();
        gained += sim.player.health - before;
    }
    // One every other tick from the potion, and the fight's own trickle.
    let from_potion = (REGENERATION_TICKS / REGENERATION_EVERY) as u32;
    assert!(gained >= from_potion && gained <= from_potion + 10, "{gained}");
    assert_eq!(sim.potion_left(Potion::Regeneration), 0);
}

#[test]
fn dying_ends_what_the_potions_were_doing() {
    let mut sim = garden();
    sim.world.set_spawn(4, 2, Some(MonsterKind::Goblin));
    sim.settle_player();
    let slot = carry(&mut sim, Item::StrengthPotion);
    sim.drink(slot);
    for _ in 0..400 {
        sim.player.health = sim.player.health.min(1);
        sim.tick_world();
        if sim.take_events().contains(&Event::Died) {
            break;
        }
    }
    assert_eq!(sim.player.health, sim.max_health(), "they died and woke");
    assert_eq!(sim.potion_left(Potion::Strength), 0);
}

#[test]
fn using_a_slot_wears_gear_drinks_potions_and_leaves_the_rest() {
    let mut sim = garden();
    sim.inventory.add(Item::CopperOre, 3);
    let sword = carry(&mut sim, Item::BronzeSword);
    let potion = carry(&mut sim, Item::StrengthPotion);
    for slot in [0, sword, potion, 20, SLOTS + 3] {
        sim.use_slot(slot);
    }
    assert_eq!(sim.inventory.count(Item::CopperOre), 3);
    assert_eq!(sim.gear.worn(crate::gear::Slot::Weapon), Some(Item::BronzeSword));
    assert!(sim.potion_left(Potion::Strength) > 0);
    assert_eq!(sim.inventory.slots()[potion], None::<Stack>);
}
