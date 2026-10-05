//! Tests for the people of the world: where they stand, how a talk
//! goes, and who wanders.

use super::*;
use crate::combat::apart;
use crate::monsters::MonsterKind;
use crate::player::{Event, Who};
use crate::terrain::Terrain;
use crate::world::Marker;

/// A 30 by 12 field with the player's start at its west end.
fn field() -> World {
    let mut world = World::new();
    for y in 0..12 {
        for x in 0..30 {
            world.set_terrain(x, y, Terrain::Grass);
        }
    }
    world.set_marker(Marker::PlayerStart, 2, 6);
    world
}

/// The field with one person living at (10, 6).
fn with(kind: PersonKind) -> Sim {
    let mut world = field();
    world.set_person(10, 6, Some(kind));
    Sim::new(world)
}

/// Tick until someone is talking to the player; at most `ticks` ticks.
fn until_talking(sim: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        if sim.saying().is_some() {
            break;
        }
        sim.tick();
    }
}

#[test]
fn everyone_has_a_name_and_something_to_say() {
    for kind in PersonKind::ALL {
        assert_eq!(PersonKind::from_name(kind.name()), Some(kind));
        assert!(!kind.label().is_empty() && !kind.calling().is_empty());
        assert!(!kind.lines().is_empty(), "{kind:?} is lost for words");
        for line in kind.lines() {
            assert!(!line.is_empty() && line.len() <= 120, "{kind:?} goes on: {line}");
        }
    }
    assert_eq!(PersonKind::from_name("dragon"), None);
    let mut labels = PersonKind::ALL.map(PersonKind::label).to_vec();
    labels.sort();
    labels.dedup();
    assert_eq!(labels.len(), PersonKind::ALL.len(), "two people share a name");
}

#[test]
fn a_person_lives_where_the_map_says_and_takes_up_their_tile() {
    let sim = with(PersonKind::Smith);
    assert_eq!(sim.people.len(), 1);
    assert_eq!((sim.people[0].kind, sim.people[0].tile, sim.people[0].moved()), (PersonKind::Smith, (10, 6), false));
    // Straight through where they stand, were they not there.
    let way = sim.way_to((16, 6));
    assert_eq!(way.last(), Some(&(16, 6)));
    assert!(!way.contains(&(10, 6)), "{way:?}");
    // And a way to them ends beside them.
    let way = sim.way_to((10, 6));
    assert!(way.last().is_some_and(|&last| last != (10, 6) && in_reach(last, (10, 6))), "{way:?}");
}

#[test]
fn asked_to_talk_the_player_walks_up_and_hears_them_out_a_line_at_a_time() {
    let mut sim = with(PersonKind::Smith);
    let lines = PersonKind::Smith.lines();
    sim.talk_to(0);
    assert!(sim.player.busy() && sim.saying().is_none(), "on the way, nothing is said yet");
    assert_eq!(sim.awaited(), Some(0));
    until_talking(&mut sim, 40);
    assert!(in_reach(sim.player.tile, (10, 6)) && sim.player.tile != (10, 6), "beside them: {:?}", sim.player.tile);
    // The last step of the way is done by the next tick, and from then
    // on talking is standing still.
    sim.tick();
    assert!(!sim.player.busy() && !sim.player.moved());
    for (i, line) in lines.iter().enumerate() {
        assert_eq!(sim.saying(), Some(Saying { person: 0, kind: PersonKind::Smith, text: line, line: i + 1, of: lines.len() }));
        assert_eq!(sim.awaited(), Some(0));
        sim.talk_on();
    }
    assert_eq!((sim.saying(), sim.awaited()), (None, None), "after the last line the talk is over");
    // Asked again, they start from the top.
    sim.talk_to(0);
    until_talking(&mut sim, 5);
    assert_eq!(sim.saying().map(|s| s.line), Some(1));
}

#[test]
fn from_beside_them_the_talk_starts_on_the_players_next_tick() {
    let mut sim = with(PersonKind::Herbalist);
    sim.player.tile = (9, 5);
    sim.player.from = sim.player.tile;
    sim.talk_to(0);
    sim.tick_player();
    assert_eq!(sim.saying().map(|s| (s.kind, s.line)), Some((PersonKind::Herbalist, 1)));
    assert_eq!((sim.player.tile, sim.player.steps), ((9, 5), 0), "not a step was taken");
    // Nobody there to ask: nothing happens.
    sim.end_talk();
    sim.talk_to(7);
    assert!(!sim.player.busy() && sim.saying().is_none());
}

#[test]
fn walking_off_ends_the_talk_and_changing_their_mind_on_the_way_means_none() {
    let mut sim = with(PersonKind::Smith);
    sim.talk_to(0);
    until_talking(&mut sim, 40);
    assert!(sim.saying().is_some());
    sim.click((3, 6));
    assert_eq!(sim.saying(), None);
    // Halfway back to them, they think better of it.
    sim.talk_to(0);
    sim.tick();
    sim.click((3, 9));
    for _ in 0..40 {
        sim.tick();
        assert_eq!(sim.saying(), None);
    }
    assert_eq!(sim.player.tile, (3, 9));
}

#[test]
fn someone_who_cannot_be_reached_is_not_talked_to() {
    let mut world = field();
    // A moat all round them.
    for (x, y) in (8..=12).flat_map(|x| (4..=8).map(move |y| (x, y))).filter(|&(x, y)| apart((x, y), (10, 6)) == 2) {
        world.set_terrain(x, y, Terrain::Water);
    }
    world.set_person(10, 6, Some(PersonKind::Smith));
    let mut sim = Sim::new(world);
    sim.talk_to(0);
    for _ in 0..60 {
        sim.tick();
    }
    assert!(sim.saying().is_none() && !sim.player.busy(), "they went as near as they could and gave up");
}

#[test]
fn a_blow_struck_at_the_player_ends_the_talk() {
    let mut world = field();
    world.set_person(10, 6, Some(PersonKind::Smith));
    world.set_spawn(9, 9, Some(MonsterKind::Goblin));
    let mut sim = Sim::new(world);
    sim.player.tile = (9, 6);
    sim.player.from = sim.player.tile;
    sim.talk_to(0);
    sim.tick_player();
    assert!(sim.saying().is_some());
    let mut events = Vec::new();
    for _ in 0..40 {
        if sim.saying().is_none() {
            break;
        }
        sim.tick();
        events.extend(sim.take_events());
    }
    assert_eq!(sim.saying(), None);
    assert!(events.iter().any(|e| matches!(e, Event::Hit { on: Who::Player, .. } | Event::Miss { on: Who::Player })), "it was a blow that ended it: {events:?}");
}

#[test]
fn a_guard_wanders_near_home_and_a_smith_stays_put() {
    let mut world = field();
    world.set_person(10, 6, Some(PersonKind::Smith));
    world.set_person(20, 6, Some(PersonKind::Guard));
    let mut sim = Sim::new(world);
    assert!(sim.waiting(), "a wanderer gives the world something to do");
    let mut seen = std::collections::HashSet::new();
    for _ in 0..600 {
        sim.tick();
        let (smith, guard) = (&sim.people[0], &sim.people[1]);
        assert_eq!((smith.kind, smith.tile, smith.moved()), (PersonKind::Smith, (10, 6), false));
        assert!(apart(guard.tile, guard.home) <= PersonKind::Guard.roam(), "they strayed to {:?}", guard.tile);
        assert!(apart(guard.tile, guard.from) <= 1 && sim.world.walkable(guard.tile.0, guard.tile.1));
        seen.insert(guard.tile);
    }
    assert!(seen.len() > 3, "the guard walked their round: {seen:?}");
    // With nobody who wanders, the world can sleep.
    assert!(!with(PersonKind::Smith).waiting());
}

#[test]
fn whoever_the_player_is_coming_to_or_talking_with_waits_for_them() {
    let mut world = field();
    world.set_person(20, 6, Some(PersonKind::Guard));
    let mut sim = Sim::new(world);
    // Let them get off their home first.
    for _ in 0..200 {
        sim.tick();
    }
    let stood = sim.people[0].tile;
    sim.talk_to(0);
    until_talking(&mut sim, 60);
    assert!(sim.saying().is_some());
    for _ in 0..200 {
        sim.tick();
        assert_eq!(sim.people[0].tile, stood, "they wandered off mid-sentence");
    }
    sim.end_talk();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..600 {
        sim.tick();
        seen.insert(sim.people[0].tile);
    }
    assert!(seen.len() > 1, "left alone again, they go back to their round");
}

/// Whether any two of the living stand on one tile.
fn crowded(sim: &Sim) -> bool {
    let mut tiles: Vec<(i32, i32)> = sim.monsters.iter().filter(|m| m.alive()).map(|m| m.tile).collect();
    tiles.extend(sim.people.iter().map(|p| p.tile));
    tiles.push(sim.player.tile);
    tiles.sort();
    tiles.windows(2).any(|w| w[0] == w[1])
}

#[test]
fn nobody_ever_shares_a_tile_with_a_person() {
    // Two goblins, a guard on their round and a smith between them and
    // the player, who fights, walks about and stops to talk, on many
    // dice: nobody ever stands on anybody.
    for seed in 0..40u64 {
        let mut world = field();
        world.set_spawn(22, 6, Some(MonsterKind::Goblin));
        world.set_spawn(21, 8, Some(MonsterKind::Goblin));
        world.set_person(20, 6, Some(PersonKind::Guard));
        world.set_person(18, 7, Some(PersonKind::Smith));
        let mut sim = Sim::new(world);
        sim.reseed(seed * 6007 + 3);
        sim.skills.set_xp(crate::skills::Skill::Vitality, crate::skills::xp_for_level(30));
        sim.player.health = sim.max_health();
        for tick in 0..400u64 {
            if !sim.player.busy() {
                match (tick / 40 + seed) % 3 {
                    0 => sim.talk_to((tick % 2) as usize),
                    1 => sim.click((16 + (tick % 9) as i32, 3 + (tick % 7) as i32)),
                    _ => {
                        if let Some(i) = sim.monsters.iter().position(|m| m.alive()) {
                            sim.attack(i);
                        }
                    }
                }
            }
            sim.tick_world();
            assert!(!crowded(&sim), "seed {seed}, world tick {tick}");
            sim.tick_player();
            assert!(!crowded(&sim), "seed {seed}, player tick {tick}");
            sim.take_events();
        }
    }
}

#[test]
fn the_map_changing_moves_people_with_it() {
    let mut sim = with(PersonKind::Smith);
    sim.talk_to(0);
    until_talking(&mut sim, 40);
    sim.world.set_person(10, 6, None);
    sim.world.set_person(4, 4, Some(PersonKind::Guard));
    sim.settle_player();
    assert_eq!(sim.saying(), None, "whoever was talking is gone");
    assert_eq!(sim.people.iter().map(|p| (p.kind, p.tile)).collect::<Vec<_>>(), vec![(PersonKind::Guard, (4, 4))]);
}

#[test]
fn the_player_is_never_stood_on_a_tile_someone_lives_on() {
    // Someone lives right on the start: the player wakes beside them.
    let mut world = field();
    world.set_person(2, 6, Some(PersonKind::Smith));
    let mut sim = Sim::new(world);
    assert!(sim.player.tile != (2, 6) && in_reach(sim.player.tile, (2, 6)), "{:?}", sim.player.tile);
    // And someone settled under their feet moves them off.
    let here = sim.player.tile;
    sim.world.set_person(here.0, here.1, Some(PersonKind::Herbalist));
    sim.settle_player();
    assert!(!sim.person_on(sim.player.tile) && in_reach(sim.player.tile, here), "{:?}", sim.player.tile);
}
