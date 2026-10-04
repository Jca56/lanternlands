//! The player, and the simulation that steps on every tick.

use std::collections::VecDeque;

use crate::path;
use crate::world::{Marker, World};

/// How far from where it was asked for a place to stand is looked for,
/// in tiles.
const STAND_SEARCH: i32 = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    /// The tile the player is on.
    pub tile: (i32, i32),
    /// The tile they were on before the last tick: the same, when they
    /// did not move.
    pub from: (i32, i32),
    /// How many steps they have ever taken.
    pub steps: u64,
    /// The tiles still to step through, and where that leads.
    path: VecDeque<(i32, i32)>,
    goal: Option<(i32, i32)>,
}

impl Player {
    /// Whether the last tick moved them.
    pub fn moved(&self) -> bool {
        self.tile != self.from
    }

    /// Where they are walking to, while they are.
    pub fn goal(&self) -> Option<(i32, i32)> {
        self.goal
    }

    /// The tiles still ahead of them.
    pub fn path(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.path.iter().copied()
    }
}

/// The world and everything in it that acts.
pub struct Sim {
    pub world: World,
    pub player: Player,
    /// How many ticks have passed.
    pub ticks: u64,
}

/// The tile nearest `near` that can be stood on, `near` itself if it
/// can. `None` when there is none within reach.
pub fn place_to_stand(world: &World, near: (i32, i32)) -> Option<(i32, i32)> {
    (0..=STAND_SEARCH).find_map(|ring| {
        // The tiles exactly `ring` steps away, nearest in a straight line first.
        let mut tiles: Vec<(i32, i32)> = (-ring..=ring).flat_map(|d| [(d, -ring), (d, ring), (-ring, d), (ring, d)]).collect();
        tiles.sort_by_key(|&(dx, dy)| (dx * dx + dy * dy, dy, dx));
        tiles.dedup();
        tiles.into_iter().map(|(dx, dy)| (near.0 + dx, near.1 + dy)).find(|&(x, y)| world.walkable(x, y))
    })
}

impl Sim {
    /// The player starts where the map says, or failing that as near
    /// the middle of the map as there is somewhere to stand.
    pub fn new(world: World) -> Sim {
        let middle = world.bounds().map_or((0, 0), |[x0, y0, x1, y1]| ((x0 + x1) / 2, (y0 + y1) / 2));
        let wanted = world.marker(Marker::PlayerStart).unwrap_or(middle);
        let tile = place_to_stand(&world, wanted).unwrap_or(wanted);
        Sim { world, player: Player { tile, from: tile, steps: 0, path: VecDeque::new(), goal: None }, ticks: 0 }
    }

    /// Set the player walking to `target`, or as near it as they can get.
    pub fn walk_to(&mut self, target: (i32, i32)) {
        self.player.path = path::find(&self.world, self.player.tile, target).into();
        self.player.goal = self.player.path.back().copied();
    }

    /// Stop the player where they stand.
    pub fn stop(&mut self) {
        self.player.path.clear();
        self.player.goal = None;
    }

    /// The world changed under the player (the map was edited): if they
    /// can no longer stand where they are, move them to the nearest
    /// place they can, and forget where they were going.
    pub fn settle_player(&mut self) {
        self.stop();
        if let Some(tile) = place_to_stand(&self.world, self.player.tile) {
            self.player.tile = tile;
        }
        self.player.from = self.player.tile;
    }

    /// One tick: the player takes the next step of their way.
    pub fn tick(&mut self) {
        self.ticks += 1;
        let player = &mut self.player;
        player.from = player.tile;
        let Some(&next) = player.path.front() else {
            player.goal = None;
            return;
        };
        let step = (next.0 - player.tile.0, next.1 - player.tile.1);
        if path::can_step(&self.world, player.tile, step) {
            player.path.pop_front();
            player.tile = next;
            player.steps += 1;
            if player.path.is_empty() {
                player.goal = None;
            }
        } else if let Some(goal) = player.goal {
            // Something is in the way that was not: find another way,
            // and take its first step on the next tick.
            self.walk_to(goal);
        }
    }
}

#[cfg(test)]
mod tests {
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
}
