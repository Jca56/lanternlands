//! Tests for the abilities.

use super::*;
use crate::combat::ATTACK_TICKS;
use crate::monsters::MonsterKind;
use crate::terrain::Terrain;
use crate::world::{Marker, World};

/// A 30 by 12 field, the player's start at its west end and a goblin
/// living at (22, 6).
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

/// The player standing beside the goblin and set on it, their own blow
/// just struck (so the next two ticks have none), the goblin whole.
fn beside(seed: u64) -> Sim {
    let mut sim = Sim::new(field());
    sim.reseed(seed);
    sim.player.tile = (21, 6);
    sim.player.from = sim.player.tile;
    sim.attack(0);
    sim.tick_player();
    sim.take_events();
    sim.monsters[0].health = MonsterKind::Goblin.stats().health;
    sim
}

/// The blows struck at monsters among some events: `Some(damage)` for
/// one that landed, `None` for a miss.
fn blows(events: &[Event]) -> Vec<Option<u32>> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Hit { on: Who::Monster(_), damage } => Some(Some(*damage)),
            Event::Miss { on: Who::Monster(_) } => Some(None),
            _ => None,
        })
        .collect()
}

#[test]
fn every_ability_has_its_own_names_and_a_time_to_wait() {
    for (i, a) in Ability::ALL.iter().enumerate() {
        assert!(a.cooldown() > 0 && !a.label().is_empty());
        assert!(a.name().chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        assert!(Ability::ALL[..i].iter().all(|b| b.name() != a.name() && b.label() != a.label()));
    }
    assert!(Ability::HeavyStrike.aimed() && Ability::Flurry.aimed() && Ability::Kick.aimed() && !Ability::Guard.aimed());
}

#[test]
fn a_heavy_strike_always_lands_for_the_most_a_blow_does_up_to_twice_that() {
    let mut seen = std::collections::HashSet::new();
    for seed in 0..200 {
        let mut sim = beside(seed);
        let most = sim.max_hit();
        sim.use_ability(Ability::HeavyStrike);
        assert!(sim.queued(Ability::HeavyStrike) && sim.swing_due());
        sim.tick_player();
        let struck = blows(&sim.take_events());
        assert_eq!(struck.len(), 1, "the strike, and no blow of their own this tick");
        let damage = struck[0].expect("it never misses");
        assert!((most..=most * 2).contains(&damage), "{damage}");
        assert!(sim.player.swung && !sim.queued(Ability::HeavyStrike));
        assert_eq!(sim.cooldown_left(Ability::HeavyStrike), Ability::HeavyStrike.cooldown());
        seen.insert(damage);
    }
    assert_eq!(seen.len(), 3, "every amount from 2 to 4 comes up: {seen:?}");
}

#[test]
fn an_ability_cannot_be_used_again_until_its_time_is_up() {
    let mut sim = beside(1);
    sim.use_ability(Ability::HeavyStrike);
    sim.tick_player();
    sim.take_events();
    for left in (1..=Ability::HeavyStrike.cooldown()).rev() {
        assert_eq!(sim.cooldown_left(Ability::HeavyStrike), left);
        sim.use_ability(Ability::HeavyStrike);
        assert!(!sim.queued(Ability::HeavyStrike), "not while it cools");
        sim.monsters[0].health = 12;
        sim.player.health = sim.max_health();
        sim.tick_world();
    }
    assert_eq!(sim.cooldown_left(Ability::HeavyStrike), 0);
    sim.use_ability(Ability::HeavyStrike);
    assert!(sim.queued(Ability::HeavyStrike));
    // The others kept their own time all along.
    assert_eq!(sim.cooldown_left(Ability::Kick), 0);
}

#[test]
fn abilities_come_on_top_of_the_players_own_blows() {
    // Thirty ticks beside a goblin that never dies: their own blows at
    // their own pace, and one more for the strike.
    let count = |with: bool| {
        let mut sim = beside(5);
        let mut struck = 0;
        for tick in 0..30 {
            sim.monsters[0].health = 12;
            sim.player.health = sim.max_health();
            if with && tick == 10 {
                sim.use_ability(Ability::HeavyStrike);
            }
            sim.tick_player();
            struck += blows(&sim.take_events()).len();
            sim.tick_world();
            sim.take_events();
        }
        struck
    };
    assert_eq!(count(false), 30 / ATTACK_TICKS as usize);
    assert_eq!(count(true), count(false) + 1);
}

#[test]
fn asked_for_on_the_way_it_waits_until_they_are_beside_the_monster() {
    let mut sim = Sim::new(field());
    sim.player.tile = (14, 6);
    sim.player.from = sim.player.tile;
    sim.attack(0);
    sim.use_ability(Ability::HeavyStrike);
    let mut used_at = None;
    for tick in 0..40 {
        sim.tick();
        let events = sim.take_events();
        if !sim.queued(Ability::HeavyStrike) {
            assert!(!sim.player.moved(), "not on the tick of a step");
            assert!(blows(&events).iter().any(|b| b.is_some_and(|d| d >= sim.max_hit())), "{events:?}");
            used_at = Some(tick);
            break;
        }
        assert_eq!(sim.cooldown_left(Ability::HeavyStrike), 0, "its time does not start until it is used");
    }
    assert!(used_at.is_some_and(|tick| tick >= 3), "{used_at:?}");
    // Letting the monster go lets the ability go with it.
    let mut sim = Sim::new(field());
    sim.player.tile = (10, 6);
    sim.attack(0);
    sim.use_ability(Ability::Kick);
    sim.walk_to((3, 6));
    sim.tick_player();
    assert!(!sim.queued(Ability::Kick));
    assert_eq!(sim.cooldown_left(Ability::Kick), 0, "and it was not spent");
}

#[test]
fn with_nobody_to_aim_at_it_says_so_or_goes_for_whoever_is_after_them() {
    let mut sim = Sim::new(field());
    sim.use_ability(Ability::Kick);
    assert_eq!(sim.take_events(), vec![Event::NoTarget]);
    assert!(!sim.queued(Ability::Kick) && !sim.acting());
    // A goblin that is after them is who it is for.
    sim.player.tile = (19, 6);
    sim.player.from = sim.player.tile;
    sim.monsters[0].angry = true;
    sim.use_ability(Ability::Kick);
    assert_eq!(sim.target(), Some(0));
    assert!(sim.queued(Ability::Kick) && sim.take_events().is_empty());
}

#[test]
fn a_kicked_monster_does_nothing_until_the_stun_wears_off() {
    let mut sim = beside(3);
    sim.tick_world();
    sim.take_events();
    sim.use_ability(Ability::Kick);
    sim.tick_player();
    let events = sim.take_events();
    assert!(events.contains(&Event::Stunned { monster: 0 }));
    assert!(blows(&events).is_empty(), "a kick does no damage");
    assert!(sim.skills.xp(Skill::Melee) > 0, "but it is practice");
    let stood = sim.monsters[0].tile;
    // Walk off: it would chase, if it could.
    sim.walk_to((17, 6));
    for _ in 0..STUN_TICKS - 1 {
        sim.tick_world();
        sim.tick_player();
        assert!(!sim.monster_swing_due(0));
        let events = sim.take_events();
        assert!(!events.iter().any(|e| matches!(e, Event::Hit { on: Who::Player, .. } | Event::Miss { on: Who::Player })), "{events:?}");
        assert_eq!(sim.monsters[0].tile, stood, "it stands where it was kicked");
    }
    for _ in 0..4 {
        sim.tick_world();
    }
    assert_ne!(sim.monsters[0].tile, stood, "and then it comes on again");
}

#[test]
fn a_flurry_is_three_blows_on_three_ticks_each_for_half_a_blows_most() {
    let mut sim = beside(11);
    sim.skills.set_xp(Skill::Melee, crate::skills::xp_for_level(30));
    let most = sim.max_hit() / 2;
    assert!(most >= 2);
    let (mut landed, mut struck) = (Vec::new(), 0);
    sim.use_ability(Ability::Flurry);
    for tick in 0..2 {
        // Clear of their own blow, which comes on the third tick.
        sim.monsters[0].health = 1000;
        assert!(sim.swing_due(), "tick {tick}");
        sim.tick_player();
        let now = blows(&sim.take_events());
        assert_eq!(now.len(), 1, "one blow of the flurry on tick {tick}");
        assert!(sim.player.swung);
        landed.extend(now.into_iter().flatten());
        struck += 1;
    }
    sim.monsters[0].health = 1000;
    sim.tick_player();
    assert_eq!(blows(&sim.take_events()).len(), 2, "the third, with their own blow beside it");
    struck += 1;
    assert_eq!(struck, FLURRY_BLOWS);
    assert!(landed.iter().all(|&d| (1..=most).contains(&d)), "{landed:?}");
    sim.tick_player();
    assert!(blows(&sim.take_events()).is_empty(), "and no fourth");
    assert_eq!(sim.cooldown_left(Ability::Flurry), Ability::Flurry.cooldown());

    // Walking off in the middle of one ends it.
    let mut sim = beside(12);
    sim.use_ability(Ability::Flurry);
    sim.tick_player();
    sim.take_events();
    sim.walk_to((10, 6));
    sim.tick_player();
    assert!(blows(&sim.take_events()).is_empty());
}

#[test]
fn a_guard_halves_what_blows_do_and_stops_the_small_ones() {
    let mut sim = Sim::new(field());
    // From standing idle with nobody about, at once.
    sim.use_ability(Ability::Guard);
    assert!(sim.acting(), "there is something for the player's tick to do");
    sim.tick_player();
    assert_eq!(sim.take_events(), vec![Event::Guarding]);
    assert_eq!(sim.guard_left(), GUARD_TICKS);
    assert!(!sim.acting());
    assert_eq!(sim.cooldown_left(Ability::Guard), Ability::Guard.cooldown());
    for left in (0..GUARD_TICKS).rev() {
        sim.tick_world();
        assert_eq!(sim.guard_left(), left);
    }

    // A goblin's blows do 1 or 2: under a guard, a 2 does 1 and a 1 is
    // stopped, and every one that would have landed is practice.
    let (mut hurt, mut blocked) = (0, 0);
    for seed in 0..40 {
        let mut sim = beside(seed);
        sim.use_ability(Ability::Guard);
        sim.tick_player();
        sim.take_events();
        let before = sim.skills.xp(Skill::Defence);
        let mut guarded = 0;
        while sim.guard_left() > 0 {
            sim.monsters[0].health = 12;
            sim.tick_world();
            for event in sim.take_events() {
                match event {
                    Event::Hit { on: Who::Player, damage } => {
                        assert_eq!(damage, 1, "never the whole of a 2");
                        hurt += 1;
                        guarded += 1;
                    }
                    Event::Blocked => {
                        blocked += 1;
                        guarded += 1;
                    }
                    Event::Miss { on: Who::Player } => guarded += 1,
                    _ => {}
                }
            }
        }
        assert_eq!(sim.skills.xp(Skill::Defence) - before, guarded * 4, "Defence for every blow met with a guard up or dodged");
    }
    assert!(hurt > 5 && blocked > 5, "hurt {hurt}, blocked {blocked}");
}

#[test]
fn dying_drops_what_was_asked_for_and_the_guard_but_not_the_waiting() {
    let mut sim = beside(2);
    sim.use_ability(Ability::Guard);
    sim.tick_player();
    sim.use_ability(Ability::Flurry);
    sim.tick_player();
    sim.walk_to((14, 6));
    sim.use_ability(Ability::Kick);
    sim.take_events();
    sim.player.health = 1;
    for _ in 0..200 {
        sim.player.health = sim.player.health.min(1);
        sim.tick_world();
        if sim.take_events().contains(&Event::Died) {
            break;
        }
    }
    assert_eq!(sim.player.health, sim.max_health(), "they died and woke");
    assert!(!sim.queued(Ability::Kick) && sim.guard_left() == 0);
    assert!(sim.cooldown_left(Ability::Guard) > 0 && sim.cooldown_left(Ability::Flurry) > 0);
}

#[test]
fn the_world_keeps_ticking_while_an_ability_cools_with_nothing_else_about() {
    let mut world = field();
    world.set_spawn(22, 6, None);
    let mut sim = Sim::new(world);
    assert!(!sim.waiting());
    sim.use_ability(Ability::Guard);
    sim.tick_player();
    for _ in 0..Ability::Guard.cooldown() {
        assert!(sim.waiting());
        sim.tick_world();
    }
    assert!(!sim.waiting());
    assert!(sim.player.activity().is_none());
}
