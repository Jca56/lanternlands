//! The player, and the simulation that steps on every tick.

use std::collections::{HashMap, VecDeque};

use crate::items::{Inventory, Item};
use crate::nodes::Node;
use crate::path;
use crate::rng::Rng;
use crate::skills::{Skill, Skills};
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
    /// The tile of the prop they are on their way to use.
    intent: Option<(i32, i32)>,
    activity: Option<Activity>,
}

/// Something the player is busy at, standing still.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    /// Working the node on tile `at`, `swing` ticks into a swing.
    Working { at: (i32, i32), swing: u32 },
}

/// Something that happened, for whoever shows the game to tell the
/// player about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A swing at the node on `at` landed, and got something or not.
    Swing { at: (i32, i32), hit: bool },
    Got { item: Item, count: u32 },
    Xp { skill: Skill, amount: u32 },
    LevelUp { skill: Skill, level: u8 },
    /// There was no room for what would have been got.
    InventoryFull,
    /// The node on `at` has given all it had.
    Spent { at: (i32, i32) },
    /// The node asks a level of a skill the player does not have.
    TooHard { skill: Skill, level: u8 },
}

/// What is left in a node that has been worked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Worked {
    pub(crate) left: u8,
    /// The tick a spent one is whole again.
    pub(crate) back_at: Option<u64>,
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

    pub fn activity(&self) -> Option<Activity> {
        self.activity
    }

    /// Whether they are doing anything at all: walking, on their way to
    /// something, or at work.
    pub fn busy(&self) -> bool {
        self.goal.is_some() || self.moved() || self.intent.is_some() || self.activity.is_some()
    }
}

/// The world and everything in it that acts.
pub struct Sim {
    pub world: World,
    pub player: Player,
    pub skills: Skills,
    pub inventory: Inventory,
    /// How many ticks have passed.
    pub ticks: u64,
    rng: Rng,
    /// Nodes that are not whole, by tile.
    pub(crate) worked: HashMap<(i32, i32), Worked>,
    events: Vec<Event>,
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
        let player = Player { tile, from: tile, steps: 0, path: VecDeque::new(), goal: None, intent: None, activity: None };
        Sim { world, player, skills: Skills::default(), inventory: Inventory::default(), ticks: 0, rng: Rng::new(0x5EED), worked: HashMap::new(), events: Vec::new() }
    }

    /// Roll different dice from here on.
    pub fn reseed(&mut self, seed: u64) {
        self.rng = Rng::new(seed);
    }

    /// What happened since this was last asked.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// The node on a tile, if what stands there is one.
    pub fn node_at(&self, tile: (i32, i32)) -> Option<Node> {
        self.world.prop_at(tile.0, tile.1)?.kind.node()
    }

    /// How many more a node on a tile will give; `None` where there is
    /// no node.
    pub fn node_left(&self, tile: (i32, i32)) -> Option<u8> {
        let node = self.node_at(tile)?;
        Some(self.worked.get(&tile).map_or(node.yields, |w| w.left))
    }

    /// Whether the node on a tile has given all it had, for now.
    pub fn is_spent(&self, tile: (i32, i32)) -> bool {
        self.node_left(tile) == Some(0)
    }

    /// Whether a tick still to come will change something even with the
    /// player standing idle: a spent node coming back.
    pub fn waiting(&self) -> bool {
        self.worked.values().any(|w| w.back_at.is_some())
    }

    /// What a press on a tile means: work the node there, or walk there.
    pub fn click(&mut self, tile: (i32, i32)) {
        if self.node_at(tile).is_none() {
            return self.walk_to(tile);
        }
        // To a node: beside it, and then to work.
        self.walk_to(tile);
        self.player.intent = Some(tile);
    }

    /// Set the player walking to `target`, or as near it as they can
    /// get. Whatever else they were at is dropped.
    pub fn walk_to(&mut self, target: (i32, i32)) {
        self.player.path = path::find(&self.world, self.player.tile, target).into();
        self.player.goal = self.player.path.back().copied();
        self.player.intent = None;
        self.player.activity = None;
    }

    /// Stop the player where they stand.
    pub fn stop(&mut self) {
        self.player.path.clear();
        self.player.goal = None;
        self.player.intent = None;
        self.player.activity = None;
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
        // What was worked may be gone, or be something else now.
        let world = &self.world;
        self.worked.retain(|&(x, y), _| world.prop_at(x, y).is_some_and(|p| p.kind.node().is_some()));
    }

    /// One tick: spent nodes come back, the player takes the next step
    /// of their way, starts what they came to do, and works at it.
    pub fn tick(&mut self) {
        self.ticks += 1;
        let now = self.ticks;
        self.worked.retain(|_, w| w.back_at.is_none_or(|at| at > now));
        self.step();
        self.arrive();
        self.work();
    }

    /// Having got where they were going, start on what they went for.
    fn arrive(&mut self) {
        if !self.player.path.is_empty() {
            return;
        }
        let Some(at) = self.player.intent.take() else {
            return;
        };
        let (dx, dy) = (at.0 - self.player.tile.0, at.1 - self.player.tile.1);
        let beside = dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0);
        let Some(node) = self.node_at(at).filter(|_| beside) else {
            return;
        };
        if self.is_spent(at) {
            self.events.push(Event::Spent { at });
        } else if self.skills.level(node.skill) < node.level {
            self.events.push(Event::TooHard { skill: node.skill, level: node.level });
        } else if !self.inventory.has_room_for(node.item) {
            self.events.push(Event::InventoryFull);
        } else {
            self.player.activity = Some(Activity::Working { at, swing: 0 });
        }
    }

    /// A tick of work: when a swing is done, roll for what it got.
    fn work(&mut self) {
        let Some(Activity::Working { at, swing }) = self.player.activity else {
            return;
        };
        let Some(node) = self.node_at(at).filter(|_| !self.is_spent(at)) else {
            // It went from under them: edited away, or spent.
            self.player.activity = None;
            return;
        };
        if swing + 1 < node.swing_ticks {
            self.player.activity = Some(Activity::Working { at, swing: swing + 1 });
            return;
        }
        self.player.activity = Some(Activity::Working { at, swing: 0 });
        let hit = self.rng.chance(node.chance_at(self.skills.level(node.skill)));
        self.events.push(Event::Swing { at, hit });
        if !hit {
            return;
        }
        if self.inventory.add(node.item, 1) > 0 {
            self.events.push(Event::InventoryFull);
            self.player.activity = None;
            return;
        }
        self.events.push(Event::Got { item: node.item, count: 1 });
        self.events.push(Event::Xp { skill: node.skill, amount: node.xp });
        if let Some(level) = self.skills.gain(node.skill, node.xp) {
            self.events.push(Event::LevelUp { skill: node.skill, level });
        }
        let worked = self.worked.entry(at).or_insert(Worked { left: node.yields, back_at: None });
        worked.left -= 1;
        if worked.left == 0 {
            worked.back_at = Some(self.ticks + node.respawn_ticks);
            self.events.push(Event::Spent { at });
            self.player.activity = None;
        } else if !self.inventory.has_room_for(node.item) {
            self.events.push(Event::InventoryFull);
            self.player.activity = None;
        }
    }

    /// The player takes the next step of their way.
    fn step(&mut self) {
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
            // and take its first step on the next tick. They are still
            // going for what they were going for.
            let intent = player.intent;
            self.walk_to(goal);
            self.player.intent = intent;
        }
    }
}

#[cfg(test)]
#[path = "player_tests.rs"]
mod tests;
