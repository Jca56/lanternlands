//! Tests for characters on disk.

use super::*;
use crate::terrain::Terrain;
use crate::world::PropKind;

fn field() -> World {
    let mut world = World::new();
    for y in 0..10 {
        for x in 0..10 {
            world.set_terrain(x, y, Terrain::Grass);
        }
    }
    world.place_prop(PropKind::Rock, 5, 2);
    world.place_prop(PropKind::Rock, 8, 8);
    world
}

fn hero() -> Character {
    Character { name: "Alva".to_owned(), played: 3723 }
}

/// A game some way in: one rock mined out, another half, some walking done.
fn played() -> Sim {
    let mut sim = Sim::new(field());
    sim.player.tile = (3, 2);
    sim.click((5, 2));
    while sim.player.busy() {
        sim.tick();
    }
    sim.click((8, 8));
    while sim.inventory.count(Item::CopperOre) < 4 {
        sim.tick();
    }
    sim.click((7, 5));
    while sim.player.busy() {
        sim.tick();
    }
    sim.take_events();
    sim
}

#[test]
fn a_game_comes_back_as_it_was_saved() {
    let sim = played();
    let saved = sim.snapshot(&hero());
    let back = Saved::decode(&saved.encode()).expect("decodes");
    assert_eq!(back, saved);
    assert_eq!(back.character, hero());

    let again = Sim::resume(field(), &back);
    assert_eq!(again.player.tile, (7, 5));
    assert_eq!(again.player.steps, sim.player.steps);
    assert_eq!(again.ticks, sim.ticks);
    assert_eq!(again.skills, sim.skills);
    assert_eq!(again.inventory, sim.inventory);
    assert_eq!((again.node_left((5, 2)), again.node_left((8, 8))), (Some(0), Some(2)));
    assert!(!again.player.busy(), "they come back standing still");
}

#[test]
fn a_spent_rock_comes_back_when_it_would_have() {
    let mut sim = played();
    // How long until the spent rock is whole, played straight through.
    let saved = sim.snapshot(&hero());
    let mut straight = 0;
    while sim.is_spent((5, 2)) {
        sim.tick();
        straight += 1;
    }
    let mut again = Sim::resume(field(), &Saved::decode(&saved.encode()).unwrap());
    let mut resumed = 0;
    while again.is_spent((5, 2)) {
        again.tick();
        resumed += 1;
    }
    assert_eq!(resumed, straight);
    assert!(straight > 0);
}

#[test]
fn the_same_game_is_always_the_same_bytes() {
    let sim = played();
    assert_eq!(sim.snapshot(&hero()).encode(), sim.snapshot(&hero()).encode());
}

#[test]
fn a_fresh_character_saves_and_resumes_too() {
    let sim = Sim::new(field());
    let fresh = Character { name: "New".to_owned(), played: 0 };
    let saved = Saved::decode(&sim.snapshot(&fresh).encode()).expect("decodes");
    assert_eq!(saved.skills.total_level(), 9);
    assert!(saved.inventory.slots().iter().all(Option::is_none));
    assert!(saved.nodes.is_empty());
    assert_eq!(Sim::resume(field(), &saved).player.tile, sim.player.tile);
}

#[test]
fn resuming_on_a_map_that_changed_puts_the_player_somewhere_they_can_stand() {
    let saved = played().snapshot(&hero());
    let mut world = field();
    // Their tile is water now, and the mined-out rock was taken away.
    world.set_terrain(7, 5, Terrain::Water);
    world.remove_prop(5, 2);
    let sim = Sim::resume(world, &saved);
    assert_ne!(sim.player.tile, (7, 5));
    assert!(sim.world.walkable(sim.player.tile.0, sim.player.tile.1));
    assert_eq!(sim.node_left((5, 2)), None);
    assert!(!sim.waiting() || sim.node_left((8, 8)).is_some());
    assert_eq!(sim.inventory.count(Item::CopperOre), 4, "what they carried is kept");
}

#[test]
fn names_not_numbers_are_what_is_stored() {
    let bytes = played().snapshot(&hero()).encode();
    let has = |name: &str| bytes.windows(name.len()).any(|w| w == name.as_bytes());
    assert!(has("Alva") && has("mining") && has("alchemy") && has("copper_ore"));
    assert!(bytes.len() < 400, "{} bytes", bytes.len());
}

#[test]
fn a_damaged_file_is_refused_with_a_reason_and_never_a_panic() {
    let good = played().snapshot(&hero()).encode();
    let refuse = |bytes: &[u8]| Saved::decode(bytes).expect_err("refused");
    assert!(refuse(b"CHNK is a map").contains("not a save file"));
    assert!(refuse(&[&good[..], &[0]].concat()).contains("left over"));
    let mut newer = good.clone();
    newer[4] = 7;
    assert!(refuse(&newer).contains("version 7"));
    for cut in 0..good.len() {
        assert!(Saved::decode(&good[..cut]).is_err(), "cut at {cut}");
    }
    let mut unknown = good.clone();
    let at = unknown.windows(10).position(|w| w == b"copper_ore").expect("the ore is named");
    unknown[at..at + 10].copy_from_slice(b"mithril_or");
    assert!(refuse(&unknown).contains("no item called \"mithril_or\""));
}

#[test]
fn odd_letters_survive_and_a_name_too_long_is_cut_between_letters() {
    let sim = Sim::new(field());
    let odd = Character { name: "Ælfgifu the Bold ⚒".to_owned(), played: 1 };
    assert_eq!(Saved::decode(&sim.snapshot(&odd).encode()).unwrap().character, odd);
    // 200 two-byte letters is 400 bytes: 127 of them fit in 255.
    let long = Character { name: "é".repeat(200), played: 0 };
    let back = Saved::decode(&sim.snapshot(&long).encode()).expect("still a save").character;
    assert_eq!(back.name, "é".repeat(127));
}

#[test]
fn a_grave_and_what_is_in_it_are_kept() {
    let mut sim = played();
    let mut held = Inventory::default();
    held.add(Item::CopperOre, 9);
    held.add(Item::GoblinTooth, 2);
    sim.grave = Some(Grave { tile: (-3, 14), items: held.clone() });
    let back = Saved::decode(&sim.snapshot(&hero()).encode()).expect("decodes");
    assert_eq!(back.grave, Some(Grave { tile: (-3, 14), items: held }));
    assert_eq!(Sim::resume(field(), &back).grave, back.grave);
    // A save from before there were graves has none.
    sim.grave = None;
    assert_eq!(Saved::decode(&sim.snapshot(&hero()).encode()).unwrap().grave, None);
}

#[test]
fn the_player_and_the_monsters_come_back_whole() {
    let mut world = field();
    world.set_spawn(1, 8, Some(crate::monsters::MonsterKind::Goblin));
    let mut sim = Sim::new(world);
    sim.player.health = 3;
    sim.monsters[0].health = 1;
    let saved = sim.snapshot(&hero());
    let mut world = field();
    world.set_spawn(1, 8, Some(crate::monsters::MonsterKind::Goblin));
    let again = Sim::resume(world, &saved);
    assert_eq!(again.player.health, again.max_health());
    assert_eq!(again.monsters.len(), 1);
    assert_eq!(again.monsters[0].health, crate::monsters::MonsterKind::Goblin.stats().health);
}
