//! Tests for fighting, health and dying.

use super::*;
use crate::items::Item;
use crate::monsters::MonsterKind;
use crate::terrain::Terrain;
use crate::world::{Marker, World};

/// A 30 by 12 field with the player's start at its west end and a
/// goblin living well to the east: (22, 6), too far to see the start.
pub(super) fn field() -> World {
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

/// Tick until `done`, gathering what happened; at most `ticks` ticks.
pub(super) fn run(sim: &mut Sim, ticks: u32, done: impl Fn(&Sim) -> bool) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        if done(sim) {
            break;
        }
        sim.tick();
        events.extend(sim.take_events());
    }
    events
}

pub(super) fn goblin() -> crate::monsters::Stats {
    MonsterKind::Goblin.stats()
}

#[test]
fn a_monster_lives_where_the_map_says_and_the_player_starts_whole() {
    let sim = Sim::new(field());
    assert_eq!(sim.monsters.len(), 1);
    assert_eq!((sim.monsters[0].tile, sim.monsters[0].health), ((22, 6), goblin().health));
    assert_eq!((sim.player.health, sim.max_health()), (20, 20));
    assert!(!sim.in_combat());
}

#[test]
fn left_alone_a_goblin_wanders_but_never_far_from_home() {
    let mut sim = Sim::new(field());
    let mut seen = std::collections::HashSet::new();
    for _ in 0..600 {
        sim.tick();
        let m = &sim.monsters[0];
        assert!(apart(m.tile, m.home) <= goblin().roam, "it strayed to {:?}", m.tile);
        assert!(apart(m.tile, m.from) <= 1, "a step at a time");
        assert!(sim.world.walkable(m.tile.0, m.tile.1));
        seen.insert(m.tile);
    }
    assert!(seen.len() > 5, "it stood on {} tiles in 600 ticks", seen.len());
    assert!(!sim.monsters[0].angry);
    assert_eq!(sim.player.health, 20, "nothing touched the player");
}

#[test]
fn a_goblin_goes_for_a_player_it_can_see_and_they_fight_back_by_themselves() {
    let mut sim = Sim::new(field());
    // Out of sight it takes no notice.
    sim.player.tile = (22 - goblin().sight - goblin().roam - 2, 6);
    run(&mut sim, 40, |_| false);
    assert!(!sim.monsters[0].angry);
    // In sight, it comes, and blows are traded with nothing clicked.
    sim.player.tile = (sim.monsters[0].tile.0 - goblin().sight, sim.monsters[0].tile.1);
    let events = run(&mut sim, 200, |sim| !sim.monsters[0].alive());
    assert!(!sim.monsters[0].alive(), "the goblin is dead");
    assert!(events.iter().any(|e| matches!(e, Event::Hit { on: Who::Monster(0), .. })), "the player struck it");
    assert!(events.iter().any(|e| matches!(e, Event::Hit { on: Who::Player, .. } | Event::Miss { on: Who::Player })), "it struck at the player");
    assert!(events.contains(&Event::Slain { monster: 0 }));
    assert!(sim.skills.xp(Skill::Melee) >= MELEE_XP * goblin().health, "experience for every point of damage");
    assert_eq!(sim.skills.xp(Skill::Vitality), VITALITY_XP * goblin().health);
}

#[test]
fn attacking_walks_to_the_monster_and_blows_come_at_the_weapons_pace() {
    let mut sim = Sim::new(field());
    sim.player.tile = (22 - goblin().leash + 1, 6);
    sim.attack(0);
    let mut swings = Vec::new();
    for tick in 0..300 {
        sim.tick();
        if sim.take_events().iter().any(|e| matches!(e, Event::Hit { on: Who::Monster(0), .. } | Event::Miss { on: Who::Monster(0) })) {
            swings.push(tick);
        }
        if !sim.monsters[0].alive() {
            break;
        }
    }
    assert!(!sim.monsters[0].alive());
    assert!(apart(sim.player.tile, sim.monsters[0].tile) <= 1, "they fought it from beside it");
    assert!(swings.len() >= 3, "{swings:?}");
    // While it stays in reach, one blow every ATTACK_TICKS.
    assert!(swings.windows(2).all(|w| w[1] - w[0] >= ATTACK_TICKS), "{swings:?}");
    assert!(swings.windows(2).any(|w| w[1] - w[0] == ATTACK_TICKS), "{swings:?}");
}

#[test]
fn a_goblin_gives_up_past_its_leash_goes_home_and_mends() {
    let mut sim = Sim::new(field());
    let home = sim.monsters[0].home;
    sim.player.tile = (home.0 - 2, home.1);
    run(&mut sim, 6, |sim| sim.monsters[0].health < goblin().health);
    assert!(sim.monsters[0].angry);
    // The player is whisked far beyond its leash.
    sim.stop();
    sim.player.tile = (home.0 - goblin().leash - 5, home.1);
    sim.monsters[0].health = 3;
    run(&mut sim, 60, |_| false);
    let m = &sim.monsters[0];
    assert!(!m.angry);
    assert!(apart(m.tile, m.home) <= goblin().roam, "back near home: {:?}", m.tile);
    assert_eq!(m.health, goblin().health, "and whole again");
}

#[test]
fn a_dead_goblin_comes_back_at_home_after_its_time() {
    let mut sim = Sim::new(field());
    sim.player.tile = (21, 6);
    sim.attack(0);
    run(&mut sim, 300, |sim| !sim.monsters[0].alive());
    let died = sim.ticks;
    // Away from where it will come back, so it does not die again at once.
    sim.stop();
    sim.player.tile = (2, 6);
    run(&mut sim, 2000, |sim| sim.monsters[0].alive());
    assert_eq!(sim.ticks - died, goblin().respawn_ticks);
    assert_eq!((sim.monsters[0].tile, sim.monsters[0].health), ((22, 6), goblin().health));
}

#[test]
fn goblins_drop_teeth_about_half_the_time_and_they_go_in_the_bag() {
    let (mut kills, mut teeth) = (0, 0);
    for seed in 0..40 {
        let mut sim = Sim::new(field());
        sim.reseed(seed);
        // Strong enough that the goblin never wins.
        sim.skills.set_xp(Skill::Vitality, crate::skills::xp_for_level(30));
        sim.player.health = sim.max_health();
        sim.player.tile = (21, 6);
        sim.attack(0);
        let events = run(&mut sim, 400, |sim| !sim.monsters[0].alive());
        kills += 1;
        let got = events.iter().filter(|e| matches!(e, Event::Got { item: Item::GoblinTooth, count: 1 })).count();
        assert_eq!(got as u32, sim.inventory.count(Item::GoblinTooth));
        teeth += got;
    }
    assert!((10..=30).contains(&teeth), "{teeth} teeth from {kills} goblins");
}

#[test]
fn health_trickles_back_in_a_fight_and_rushes_back_out_of_one() {
    let mut sim = Sim::new(World::new());
    let max = sim.max_health();
    sim.player.health = 1;
    sim.last_fight = Some(sim.ticks);
    for _ in 0..COMBAT_TICKS - 1 {
        sim.tick_world();
    }
    assert!(sim.player.health <= 3, "still in the fight: only a trickle ({})", sim.player.health);
    assert!(sim.waiting(), "a hurt player keeps the world ticking");
    let mut ticks = 0;
    while sim.player.health < max {
        sim.tick_world();
        ticks += 1;
    }
    assert!((15..=25).contains(&ticks), "full again {ticks} ticks after the fight");
    sim.tick_world();
    assert_eq!(sim.player.health, max, "and no more than full");
    assert!(!sim.waiting());
}

#[test]
fn more_vitality_is_more_health_and_more_melee_hits_harder() {
    let mut sim = Sim::new(World::new());
    assert_eq!((sim.max_health(), sim.max_hit()), (20, 2));
    sim.skills.set_xp(Skill::Vitality, crate::skills::xp_for_level(11));
    sim.skills.set_xp(Skill::Melee, crate::skills::xp_for_level(12));
    assert_eq!((sim.max_health(), sim.max_hit()), (80, 6));
}

#[test]
fn the_chance_to_hit_follows_the_levels_and_stays_between_its_ends() {
    assert_eq!(hit_chance(1, 1), 0.60);
    assert!(hit_chance(10, 1) > hit_chance(5, 1) && hit_chance(1, 10) < hit_chance(1, 5));
    assert_eq!((hit_chance(50, 1), hit_chance(1, 50)), (0.95, 0.10));
}

/// A player set upon by a goblin they cannot beat: one health left and
/// too weak to land a blow. Returns the game just after they died.
fn killed_carrying(ore: u32) -> Sim {
    let mut sim = Sim::new(field());
    sim.inventory.add(Item::CopperOre, ore);
    sim.player.tile = (21, 6);
    for _ in 0..500 {
        sim.player.health = sim.player.health.min(1);
        sim.tick_world();
        if sim.take_events().contains(&Event::Died) {
            return sim;
        }
    }
    panic!("the goblin never killed them");
}

#[test]
fn dying_leaves_a_grave_with_the_bag_and_wakes_the_player_at_the_start() {
    let sim = killed_carrying(7);
    assert_eq!(sim.player.tile, (2, 6), "at the start");
    assert_eq!(sim.player.health, sim.max_health(), "whole");
    assert!(sim.inventory.is_empty());
    let grave = sim.grave.as_ref().expect("a grave");
    assert_eq!((grave.tile, grave.items.count(Item::CopperOre)), ((21, 6), 7));
    assert!(sim.monsters.iter().all(|m| !m.angry), "the goblin lets them be");
    assert!(!sim.player.busy() && !sim.in_combat());
}

#[test]
fn walking_back_to_the_grave_and_clicking_it_gives_everything_back() {
    let mut sim = killed_carrying(7);
    // Keep the goblin out of it.
    sim.monsters.clear();
    sim.click((21, 6));
    let events = run(&mut sim, 100, |sim| !sim.player.busy());
    assert!(events.contains(&Event::GraveOpened { all: true }));
    assert_eq!(sim.inventory.count(Item::CopperOre), 7);
    assert!(sim.grave.is_none(), "the grave is gone once it is empty");
}

#[test]
fn dying_again_before_reaching_the_grave_loses_what_was_in_it() {
    let mut sim = killed_carrying(7);
    sim.inventory.add(Item::GoblinTooth, 2);
    sim.player.tile = (23, 7);
    let mut lost = false;
    for _ in 0..500 {
        sim.player.health = sim.player.health.min(1);
        sim.tick_world();
        let events = sim.take_events();
        lost |= events.contains(&Event::GraveLost);
        if events.contains(&Event::Died) {
            break;
        }
    }
    assert!(lost, "the old grave crumbled");
    let grave = sim.grave.as_ref().expect("a new grave");
    assert_eq!((grave.items.count(Item::CopperOre), grave.items.count(Item::GoblinTooth)), (0, 2), "only what they carried this time");
}

#[test]
fn dying_with_nothing_leaves_no_grave() {
    let sim = killed_carrying(0);
    assert!(sim.grave.is_none());
}

#[test]
fn a_grave_gives_back_what_fits_and_keeps_the_rest() {
    let mut sim = killed_carrying(7);
    sim.monsters.clear();
    // The bag is full of something else by the time they get back.
    while sim.inventory.has_room_for(Item::GoblinTooth) {
        sim.inventory.add(Item::GoblinTooth, u32::MAX);
    }
    sim.click((21, 6));
    let events = run(&mut sim, 100, |sim| !sim.player.busy());
    assert!(events.contains(&Event::GraveOpened { all: false }));
    assert_eq!(sim.grave.as_ref().map(|g| g.items.count(Item::CopperOre)), Some(7));
}

#[test]
fn the_worlds_tick_and_the_players_keep_their_own_time() {
    let mut sim = Sim::new(field());
    sim.walk_to((6, 6));
    // The world alone: monsters move, the player does not.
    for _ in 0..5 {
        sim.tick_world();
    }
    assert_eq!((sim.ticks, sim.player.tile), (5, (2, 6)));
    // The player alone: they walk, and the world's clock stands.
    for _ in 0..4 {
        sim.tick_player();
    }
    assert_eq!((sim.ticks, sim.player.tile), (5, (6, 6)));
}

#[test]
fn editing_the_map_puts_the_monsters_where_it_now_says() {
    let mut sim = Sim::new(field());
    sim.world.set_spawn(22, 6, None);
    sim.world.set_spawn(10, 3, Some(MonsterKind::Goblin));
    sim.world.set_spawn(15, 9, Some(MonsterKind::Goblin));
    sim.settle_player();
    let homes: Vec<_> = sim.monsters.iter().map(|m| m.home).collect();
    assert_eq!(homes, vec![(10, 3), (15, 9)]);
}
