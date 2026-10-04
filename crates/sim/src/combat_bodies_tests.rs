//! Tests for who may stand where, when blows fall, and what the game
//! is told about swings.

use super::tests::{field, goblin, run};
use super::*;
use crate::monsters::MonsterKind;

/// A blow the player struck at a monster this tick, landed or not.
fn struck(events: &[Event]) -> bool {
    events.iter().any(|e| matches!(e, Event::Hit { on: Who::Monster(_), .. } | Event::Miss { on: Who::Monster(_) }))
}

/// Whether any two of the living stand on one tile.
fn crowded(sim: &Sim) -> bool {
    let mut tiles: Vec<(i32, i32)> = sim.monsters.iter().filter(|m| m.alive()).map(|m| m.tile).collect();
    tiles.push(sim.player.tile);
    tiles.sort();
    tiles.windows(2).any(|w| w[0] == w[1])
}

#[test]
fn a_tile_holds_one_body_through_whole_fights() {
    // Three goblins living side by side, the player coming at them from
    // every distance, on many dice: nobody ever stands on anybody.
    for seed in 0..60u64 {
        let mut world = field();
        world.set_spawn(21, 7, Some(MonsterKind::Goblin));
        world.set_spawn(22, 5, Some(MonsterKind::Goblin));
        let mut sim = Sim::new(world);
        sim.reseed(seed * 7919 + 1);
        sim.skills.set_xp(Skill::Vitality, crate::skills::xp_for_level(30));
        sim.player.health = sim.max_health();
        sim.player.tile = (10 + (seed % 9) as i32, 2 + (seed % 7) as i32);
        sim.player.from = sim.player.tile;
        for tick in 0..400 {
            if !sim.player.busy()
                && let Some(i) = sim.monsters.iter().position(|m| m.alive())
            {
                sim.attack(i);
            }
            sim.tick_world();
            assert!(!crowded(&sim), "seed {seed}, world tick {tick}: {:?} and {:?}", sim.player.tile, sim.monsters.iter().map(|m| m.tile).collect::<Vec<_>>());
            sim.tick_player();
            assert!(!crowded(&sim), "seed {seed}, player tick {tick}: {:?} and {:?}", sim.player.tile, sim.monsters.iter().map(|m| m.tile).collect::<Vec<_>>());
            for m in &sim.monsters {
                assert!(apart(m.tile, m.from) <= 1 && sim.world.walkable(m.tile.0, m.tile.1));
            }
            sim.take_events();
        }
    }
}

#[test]
fn the_players_way_goes_round_a_monster_and_stops_beside_one_on_its_end() {
    let mut sim = Sim::new(field());
    sim.player.tile = (18, 6);
    // Straight through where the goblin stands, were it not there.
    let way = sim.way_to((26, 6));
    assert_eq!(way.last(), Some(&(26, 6)));
    assert!(!way.contains(&(22, 6)), "{way:?}");
    assert_eq!(way.len(), 8, "going round costs no more steps on open ground: {way:?}");
    // Asked onto the goblin itself, they stop beside it.
    let way = sim.way_to((22, 6));
    assert_eq!(way.last(), Some(&(21, 6)), "{way:?}");
    // A monster that walks into the way is walked round, not through.
    sim.walk_to((26, 6));
    let next = sim.player.path().next().unwrap();
    sim.monsters[0].tile = next;
    sim.tick_player();
    assert_ne!(sim.player.tile, next);
    for _ in 0..12 {
        sim.tick_player();
        assert_ne!(sim.player.tile, sim.monsters[0].tile);
    }
    assert_eq!(sim.player.tile, (26, 6), "and they still get where they were going");
}

#[test]
fn a_monster_the_player_wakes_on_steps_off_before_anything_else() {
    let mut sim = Sim::new(field());
    sim.player.tile = sim.monsters[0].tile;
    sim.player.from = sim.player.tile;
    sim.tick_world();
    assert_ne!(sim.monsters[0].tile, sim.player.tile);
    assert!(apart(sim.monsters[0].tile, sim.player.tile) == 1);
    assert!(sim.take_events().is_empty(), "no blow from on top of them");
}

#[test]
fn a_dead_goblin_waits_to_come_back_while_someone_stands_on_its_home() {
    let mut sim = Sim::new(field());
    let home = sim.monsters[0].home;
    sim.monsters[0].back_at = Some(sim.ticks + 2);
    sim.player.tile = home;
    sim.player.from = home;
    run(&mut sim, 20, |_| false);
    assert!(!sim.monsters[0].alive(), "not onto the player");
    sim.player.tile = (home.0 - 12, home.1);
    sim.player.from = sim.player.tile;
    sim.tick();
    assert!(sim.monsters[0].alive(), "the tick they step off, it is back");
    assert_eq!(sim.monsters[0].tile, home);
}

#[test]
fn the_first_blow_lands_when_they_have_got_there() {
    // Set at it on the tick of their last step, the blow comes with the
    // next tick, when the step is done. Set at it standing still (it
    // walked up to them), the blow comes at once.
    let (mut walked_up, mut stood) = (0, 0);
    for far in 2..=goblin().leash {
        let mut sim = Sim::new(field());
        sim.player.tile = (22 - far, 6);
        sim.player.from = sim.player.tile;
        sim.attack(0);
        for _ in 0..60 {
            sim.tick();
            let events = sim.take_events();
            if !matches!(sim.player.activity(), Some(Activity::Fighting { .. })) {
                assert!(!struck(&events) && !sim.player.swung, "no blow on the way");
                continue;
            }
            if sim.player.moved() {
                assert!(!struck(&events) && !sim.player.swung, "not while the last step is being taken");
                assert!(sim.swing_due(), "but next");
                sim.tick_player();
                assert!(struck(&sim.take_events()) && sim.player.swung && !sim.player.moved());
                walked_up += 1;
            } else {
                assert!(struck(&events) && sim.player.swung);
                stood += 1;
            }
            break;
        }
    }
    assert_eq!(walked_up + stood, goblin().leash - 1, "every one of them came to blows");
    assert!(walked_up > 0, "walked up {walked_up}, stood {stood}");

    // Standing beside it already, the blow is the very next tick.
    let mut sim = Sim::new(field());
    sim.player.tile = (21, 6);
    sim.player.from = sim.player.tile;
    sim.attack(0);
    assert!(sim.swing_due());
    sim.tick_player();
    assert!(struck(&sim.take_events()) && sim.player.swung);
}

#[test]
fn swung_marks_the_ticks_of_the_blows_and_due_the_tick_before_each() {
    let mut sim = Sim::new(field());
    sim.player.tile = (21, 6);
    sim.player.from = sim.player.tile;
    sim.attack(0);
    let mut blows = 0;
    for _ in 0..40 {
        // A goblin that never dies and never hurts.
        sim.monsters[0].health = goblin().health;
        sim.player.health = sim.max_health();
        let due = sim.swing_due();
        sim.tick_player();
        let was = struck(&sim.take_events());
        assert_eq!(sim.player.swung, was);
        assert_eq!(due, was, "a blow comes exactly when one was due");
        blows += was as u32;
        sim.tick_world();
        sim.take_events();
    }
    assert_eq!(blows, 40_u32.div_ceil(ATTACK_TICKS));
    // Walking off, nothing is due and nothing was swung.
    sim.walk_to((10, 6));
    sim.tick_player();
    assert!(!sim.swing_due() && !sim.player.swung);
}

#[test]
fn a_monsters_blow_is_due_the_tick_before_it_swings() {
    let mut sim = Sim::new(field());
    sim.player.tile = (21, 6);
    sim.player.from = sim.player.tile;
    let mut swings = 0;
    for _ in 0..60 {
        sim.player.health = sim.max_health();
        sim.monsters[0].health = goblin().health;
        let due = sim.monster_swing_due(0);
        sim.tick_world();
        let swung = sim.monsters[0].swung_at == Some(sim.ticks);
        if sim.ticks > 1 {
            assert_eq!(due, swung, "tick {}", sim.ticks);
        }
        swings += swung as u32;
    }
    assert!(swings >= 14, "{swings}");
}

#[test]
fn the_target_is_the_monster_gone_for_or_fought_and_nobody_else() {
    let mut sim = Sim::new(field());
    assert_eq!(sim.target(), None);
    sim.player.tile = (22 - goblin().leash, 6);
    sim.attack(0);
    assert_eq!(sim.target(), Some(0), "on the way to it");
    run(&mut sim, 40, |sim| matches!(sim.player.activity(), Some(Activity::Fighting { .. })));
    assert_eq!(sim.target(), Some(0), "and at it");
    sim.walk_to((5, 6));
    assert_eq!(sim.target(), None, "walking off lets it go");
    sim.attack(0);
    run(&mut sim, 300, |sim| !sim.monsters[0].alive());
    assert_eq!(sim.target(), None, "a dead one is nobody's target");
}
