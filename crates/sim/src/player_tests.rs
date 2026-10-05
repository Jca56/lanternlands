//! Tests for the player and the ticking simulation.

use super::*;
use crate::terrain::Terrain;
use crate::world::PropKind;

fn field(width: i32, height: i32) -> World {
    let mut world = World::new();
    for y in 0..height {
        for x in 0..width {
            world.set_terrain(x, y, Terrain::Grass);
        }
    }
    world
}

#[test]
fn the_player_starts_on_the_maps_start() {
    let mut world = field(10, 10);
    world.set_marker(Marker::PlayerStart, 7, 2);
    assert_eq!(Sim::new(world).player.tile, (7, 2));
}

#[test]
fn with_no_start_they_stand_near_the_middle_and_never_in_a_tree() {
    // A field's chunk spans 32 tiles, so its middle is (16, 16): off
    // this field. The nearest ground to that is its corner.
    assert_eq!(Sim::new(field(10, 10)).player.tile, (9, 9));
    // Forty tiles span two chunks each way: the middle is (32, 32).
    let mut world = field(40, 40);
    world.place_prop(PropKind::Tree, 32, 32);
    let sim = Sim::new(world);
    assert_ne!(sim.player.tile, (32, 32));
    assert!(sim.world.walkable(sim.player.tile.0, sim.player.tile.1));
    assert_eq!(path::find(&sim.world, sim.player.tile, (32, 32)), vec![], "they are right beside it");
}

#[test]
fn a_start_that_cannot_be_stood_on_puts_them_beside_it() {
    let mut world = field(10, 10);
    world.set_marker(Marker::PlayerStart, 5, 5);
    world.set_terrain(5, 5, Terrain::Water);
    let sim = Sim::new(world);
    assert_eq!(sim.player.tile, (5, 4));
}

#[test]
fn an_empty_world_does_not_stop_the_game_starting() {
    let sim = Sim::new(World::new());
    assert_eq!(sim.player.tile, (0, 0));
}

#[test]
fn walking_takes_one_tile_a_tick_and_stops_at_the_goal() {
    let mut sim = Sim::new(field(10, 10));
    sim.player.tile = (0, 0);
    sim.walk_to((4, 2));
    assert_eq!(sim.player.goal(), Some((4, 2)));
    let mut seen = Vec::new();
    for _ in 0..6 {
        sim.tick();
        seen.push((sim.player.tile, sim.player.moved()));
    }
    assert_eq!(seen, vec![((1, 1), true), ((2, 2), true), ((3, 2), true), ((4, 2), true), ((4, 2), false), ((4, 2), false)]);
    assert_eq!((sim.player.steps, sim.ticks, sim.player.goal()), (4, 6, None));
}

#[test]
fn a_new_goal_mid_walk_turns_them_from_where_they_are() {
    let mut sim = Sim::new(field(10, 10));
    sim.player.tile = (0, 0);
    sim.walk_to((9, 0));
    sim.tick();
    sim.tick();
    sim.walk_to((2, 5));
    for _ in 0..5 {
        sim.tick();
    }
    assert_eq!(sim.player.tile, (2, 5));
}

#[test]
fn a_way_that_gets_blocked_is_found_again() {
    let mut sim = Sim::new(field(10, 3));
    sim.player.tile = (0, 1);
    sim.walk_to((9, 1));
    sim.tick();
    // A wall of trees goes up ahead, with a gap at the bottom.
    for y in 0..2 {
        sim.world.place_prop(PropKind::Tree, 2, y);
    }
    for _ in 0..20 {
        sim.tick();
    }
    assert_eq!(sim.player.tile, (9, 1), "they went round");
}

#[test]
fn asked_to_walk_into_water_they_walk_to_the_shore() {
    let mut world = field(10, 3);
    for y in 0..3 {
        world.set_terrain(6, y, Terrain::Water);
    }
    let mut sim = Sim::new(world);
    sim.player.tile = (0, 1);
    sim.walk_to((6, 1));
    assert_eq!(sim.player.goal(), Some((5, 1)));
    for _ in 0..10 {
        sim.tick();
    }
    assert_eq!(sim.player.tile, (5, 1));
}

#[test]
fn settling_moves_a_player_off_ground_that_was_edited_away() {
    let mut sim = Sim::new(field(10, 10));
    sim.player.tile = (4, 4);
    sim.walk_to((9, 9));
    sim.world.place_prop(PropKind::Rock, 4, 4);
    sim.settle_player();
    assert_ne!(sim.player.tile, (4, 4));
    assert!(sim.world.walkable(sim.player.tile.0, sim.player.tile.1));
    assert_eq!(sim.player.goal(), None);
    assert!(!sim.player.moved());
    // On good ground, settling leaves them be.
    let before = sim.player.tile;
    sim.settle_player();
    assert_eq!(sim.player.tile, before);
}

// ---- working nodes -------------------------------------------------

use crate::world::PropKind as P;

/// A field with a rock at (5, 2) and the player two tiles west of it.
fn at_the_rock() -> Sim {
    let mut world = field(10, 5);
    world.place_prop(P::Rock, 5, 2);
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

#[test]
fn clicking_a_rock_walks_beside_it_and_mines_it_out() {
    let mut sim = at_the_rock();
    sim.click((5, 2));
    let events = run(&mut sim);
    assert_eq!(sim.player.tile, (4, 2), "beside the rock, not on it");
    let node = P::Rock.node().unwrap();
    assert_eq!(sim.inventory.count(Item::CopperOre), node.yields as u32);
    assert_eq!(sim.skills.xp(Skill::Mining), node.xp * node.yields as u32);
    assert!(sim.is_spent((5, 2)));
    assert_eq!(events.last(), Some(&Event::Spent { at: (5, 2) }));
    let got = events.iter().filter(|e| matches!(e, Event::Got { item: Item::CopperOre, count: 1 })).count();
    let hits = events.iter().filter(|e| matches!(e, Event::Swing { hit: true, .. })).count();
    assert_eq!((got, hits), (3, 3));
    assert!(events.contains(&Event::LevelUp { skill: Skill::Mining, level: 2 }), "three ores is level 2");
}

#[test]
fn a_swing_takes_its_ticks_and_misses_cost_time_not_ore() {
    let node = P::Rock.node().unwrap();
    // The ticks on which swings landed, mining the rock out from
    // right beside it with the dice seeded `seed`.
    let swings = |seed| {
        let mut sim = at_the_rock();
        sim.reseed(seed);
        sim.player.tile = (4, 2);
        sim.click((5, 2));
        let mut landed = Vec::new();
        while landed.is_empty() || sim.player.busy() {
            sim.tick();
            if sim.take_events().iter().any(|e| matches!(e, Event::Swing { .. })) {
                landed.push(sim.ticks as u32);
            }
        }
        assert_eq!(sim.inventory.count(Item::CopperOre), node.yields as u32, "seed {seed}: a miss costs no ore");
        landed
    };
    let runs: Vec<Vec<u32>> = (0..20).map(swings).collect();
    for landed in &runs {
        // Work starts on the first tick; a swing lands every so many.
        assert_eq!(landed[0], node.swing_ticks);
        assert!(landed.windows(2).all(|w| w[1] - w[0] == node.swing_ticks), "{landed:?}");
    }
    assert!(runs.iter().any(|landed| landed.len() > node.yields as usize), "no seed of twenty ever missed");
    assert!(runs.iter().any(|landed| landed.len() == node.yields as usize), "no seed of twenty hit every time");
}

#[test]
fn a_spent_rock_comes_back_after_its_time() {
    let mut sim = at_the_rock();
    sim.click((5, 2));
    run(&mut sim);
    let spent_at = sim.ticks;
    assert!(sim.waiting());
    assert_eq!(sim.node_left((5, 2)), Some(0));
    // Asked to mine it now, they are told it is spent and do nothing.
    sim.click((5, 2));
    assert_eq!(run(&mut sim), vec![Event::Spent { at: (5, 2) }]);
    let node = P::Rock.node().unwrap();
    while sim.ticks < spent_at + node.respawn_ticks - 1 {
        sim.tick();
    }
    assert!(sim.is_spent((5, 2)), "not a tick early");
    sim.tick();
    assert_eq!(sim.node_left((5, 2)), Some(node.yields));
    assert!(!sim.waiting());
}

#[test]
fn walking_off_drops_the_work() {
    let mut sim = at_the_rock();
    sim.click((5, 2));
    sim.tick();
    sim.tick();
    assert!(matches!(sim.player.activity(), Some(Activity::Working { at: (5, 2), .. })));
    sim.click((0, 0));
    assert_eq!(sim.player.activity(), None);
    let ore = sim.inventory.count(Item::CopperOre);
    run(&mut sim);
    assert_eq!(sim.player.tile, (0, 0));
    assert_eq!(sim.inventory.count(Item::CopperOre), ore, "no ore from a rock they left");
}

#[test]
fn a_full_bag_stops_the_work_before_a_swing_is_wasted() {
    let mut sim = at_the_rock();
    // With one item in the game, a full bag is every slot a stack
    // filled to the brim.
    while sim.inventory.has_room_for(Item::CopperOre) {
        sim.inventory.add(Item::CopperOre, u32::MAX);
    }
    assert!(sim.inventory.slots().iter().all(Option::is_some));
    sim.click((5, 2));
    assert_eq!(run(&mut sim), vec![Event::InventoryFull]);
    assert_eq!(sim.node_left((5, 2)), Some(3), "the rock was not touched");
}

#[test]
fn the_same_seed_mines_the_same_and_another_seed_does_not() {
    let mine = |seed| {
        let mut sim = at_the_rock();
        sim.reseed(seed);
        sim.click((5, 2));
        run(&mut sim);
        sim.ticks
    };
    assert_eq!(mine(1), mine(1));
    assert!((2..40).any(|seed| mine(seed) != mine(1)), "every seed took the same time");
}

#[test]
fn a_node_edited_away_is_forgotten() {
    let mut sim = at_the_rock();
    sim.click((5, 2));
    run(&mut sim);
    assert!(sim.waiting());
    sim.world.remove_prop(5, 2);
    sim.settle_player();
    assert!(!sim.waiting());
    assert_eq!(sim.node_left((5, 2)), None);
    // A new rock stood there is whole.
    sim.world.place_prop(P::Rock, 5, 2);
    assert_eq!(sim.node_left((5, 2)), Some(3));
}

#[test]
fn a_rock_that_cannot_be_reached_is_not_mined_from_afar() {
    let mut world = field(10, 5);
    // The rock on an island.
    for (x, y) in [(6, 1), (7, 1), (8, 1), (6, 2), (8, 2), (6, 3), (7, 3), (8, 3)] {
        world.set_terrain(x, y, Terrain::Water);
    }
    world.place_prop(P::Rock, 7, 2);
    let mut sim = Sim::new(world);
    sim.player.tile = (0, 2);
    sim.click((7, 2));
    assert_eq!(run(&mut sim), vec![]);
    assert_eq!(sim.player.tile, (5, 2), "as near as the water lets them");
    assert_eq!(sim.inventory.count(Item::CopperOre), 0);
}

#[test]
fn at_a_rock_swung_marks_only_the_ticks_a_swing_ends_and_due_the_tick_before() {
    let mut sim = at_the_rock();
    sim.click((5, 2));
    let (mut swings, mut ticks) = (0, 0);
    for _ in 0..200 {
        let due = sim.swing_due();
        sim.tick();
        let swung = sim.take_events().iter().any(|e| matches!(e, Event::Swing { .. }));
        assert_eq!(sim.player.swung, swung);
        assert_eq!(due, swung, "a swing ends exactly when one was due");
        swings += swung as u32;
        ticks += matches!(sim.player.activity(), Some(Activity::Working { .. })) as u32;
        if !sim.player.busy() {
            break;
        }
    }
    assert!(swings >= 3, "{swings}");
    assert!(ticks >= swings * 2, "most ticks at the rock are not swings: {swings} in {ticks}");
    assert!(sim.player.swung, "the swing that emptied it still counts");
    sim.tick_player();
    assert!(!sim.player.swung && !sim.swing_due());
}

#[test]
fn what_the_player_is_set_on_using_is_known_from_the_click_until_they_are_done_with_it() {
    let mut sim = at_the_rock();
    assert_eq!(sim.using(), None);
    // A plain walk is set on nothing.
    sim.click((1, 1));
    assert_eq!(sim.using(), None);
    // A rock: on the way to it, and at work on it, until it is spent.
    sim.click((5, 2));
    let mut ticks = 0;
    while !sim.is_spent((5, 2)) {
        assert_eq!(sim.using(), Some((5, 2)), "tick {ticks}");
        sim.tick();
        ticks += 1;
        assert!(ticks < 2000, "the rock was never mined out");
    }
    assert_eq!(sim.using(), None);
    // A station: on the way to it, and for as long as they stand at it.
    sim.world.place_prop(P::Furnace, 8, 2);
    sim.click((8, 2));
    assert_eq!(sim.using(), Some((8, 2)));
    run(&mut sim);
    assert!(sim.station().is_some());
    assert_eq!(sim.using(), Some((8, 2)));
    sim.leave_station();
    assert_eq!(sim.using(), None);
    // The grave: while they go back for it.
    sim.grave = Some(crate::combat::Grave { tile: (1, 3), items: Default::default() });
    sim.click((1, 3));
    assert_eq!(sim.using(), Some((1, 3)));
    run(&mut sim);
    assert_eq!((sim.using(), sim.grave.is_some()), (None, false));
}
