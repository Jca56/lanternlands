//! The player, and the simulation that steps on every tick.

use std::collections::{HashMap, VecDeque};

use crate::combat::Grave;
use crate::items::{Inventory, Item};
use crate::monsters::Monster;
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
    pub health: u32,
    /// Whether their last tick was a swing: a blow at a monster, or
    /// the end of a swing at a node.
    pub swung: bool,
    /// The tiles still to step through, and where that leads.
    pub(crate) path: VecDeque<(i32, i32)>,
    pub(crate) goal: Option<(i32, i32)>,
    /// What they are on their way to do.
    pub(crate) intent: Option<Intent>,
    pub(crate) activity: Option<Activity>,
}

/// Something the player means to do once they are in reach of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Intent {
    /// Use what is on a tile: work a node, open the grave.
    Use((i32, i32)),
    /// Fight a monster.
    Attack(usize),
}

/// Something the player is busy at, standing still.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    /// Working the node on tile `at`, `swing` ticks into a swing.
    Working { at: (i32, i32), swing: u32 },
    /// Fighting a monster, `cooldown` ticks from the next blow.
    Fighting { target: usize, cooldown: u32 },
}

/// Who a blow was aimed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    Player,
    /// A monster, by its place among the simulation's.
    Monster(usize),
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
    /// A blow landed, for this much.
    Hit { on: Who, damage: u32 },
    /// A blow missed.
    Miss { on: Who },
    /// A monster was killed.
    Slain { monster: usize },
    /// The player died, and woke at the start.
    Died,
    /// An older grave crumbled, with what was in it.
    GraveLost,
    /// The grave gave back what it held: `all` of it, or what there
    /// was room for.
    GraveOpened { all: bool },
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
    /// How many of the world's ticks have passed.
    pub ticks: u64,
    /// Nodes that are not whole, by tile.
    pub(crate) worked: HashMap<(i32, i32), Worked>,
    /// Every monster on the map, in the order of the map's spawns.
    pub monsters: Vec<Monster>,
    /// Where the player died, and what they carried.
    pub grave: Option<Grave>,
    /// The world tick of the last blow given or taken.
    pub(crate) last_fight: Option<u64>,
    pub(crate) rng: Rng,
    pub(crate) events: Vec<Event>,
}

/// Where a player with nowhere else to be appears: where the map says,
/// or failing that as near the middle of it as there is somewhere to
/// stand.
pub(crate) fn start_tile(world: &World) -> (i32, i32) {
    let middle = world.bounds().map_or((0, 0), |[x0, y0, x1, y1]| ((x0 + x1) / 2, (y0 + y1) / 2));
    let wanted = world.marker(Marker::PlayerStart).unwrap_or(middle);
    place_to_stand(world, wanted).unwrap_or(wanted)
}

/// A monster on every tile the map says one lives on.
fn monsters_of(world: &World) -> Vec<Monster> {
    world.spawns().into_iter().map(|(kind, x, y)| Monster::new(kind, (x, y))).collect()
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
    /// The player starts where the map says, whole and with nothing.
    pub fn new(world: World) -> Sim {
        let tile = start_tile(&world);
        let player = Player { tile, from: tile, steps: 0, health: 0, swung: false, path: VecDeque::new(), goal: None, intent: None, activity: None };
        let monsters = monsters_of(&world);
        let mut sim = Sim { world, player, skills: Skills::default(), inventory: Inventory::default(), ticks: 0, worked: HashMap::new(), monsters, grave: None, last_fight: None, rng: Rng::new(0x5EED), events: Vec::new() };
        sim.player.health = sim.max_health();
        sim
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

    /// Whether a tick of the world still to come will change something
    /// even with the player standing idle: a spent node coming back, a
    /// monster going about, health mending.
    pub fn waiting(&self) -> bool {
        self.worked.values().any(|w| w.back_at.is_some()) || !self.monsters.is_empty() || self.player.health < self.max_health()
    }

    /// Whether anything other than the player moved or struck on the
    /// world's last tick: something to watch.
    pub fn stirring(&self) -> bool {
        self.monsters.iter().any(|m| m.alive() && (m.moved() || m.swung_at == Some(self.ticks)))
    }

    /// What a press on a tile means: use what is there (a node, the
    /// grave), or walk there.
    pub fn click(&mut self, tile: (i32, i32)) {
        let usable = self.node_at(tile).is_some() || self.grave.as_ref().is_some_and(|g| g.tile == tile);
        self.walk_to(tile);
        if usable {
            // To it, and then to use it.
            self.player.intent = Some(Intent::Use(tile));
        }
    }

    /// Set the player walking to `target`, or as near it as they can
    /// get. Whatever else they were at is dropped.
    pub fn walk_to(&mut self, target: (i32, i32)) {
        self.player.path = self.way_to(target).into();
        self.player.goal = self.player.path.back().copied();
        self.player.intent = None;
        self.player.activity = None;
    }

    /// Whether a living monster stands on `tile`.
    pub(crate) fn monster_on(&self, tile: (i32, i32)) -> bool {
        self.monsters.iter().any(|m| m.alive() && m.tile == tile)
    }

    /// The player's way to `to`, round every monster: a tile holds one
    /// body. Where a monster stands on `to` itself, the way stops
    /// beside it.
    pub(crate) fn way_to(&self, to: (i32, i32)) -> Vec<(i32, i32)> {
        let mut way = path::find_around(&self.world, self.player.tile, to, &|tile| self.monster_on(tile));
        if way.last().is_some_and(|&last| self.monster_on(last)) {
            way.pop();
        }
        way
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
        // What was worked may be gone, or be something else now, and
        // the monsters live where the map now says.
        let world = &self.world;
        self.worked.retain(|&(x, y), _| world.prop_at(x, y).is_some_and(|p| p.kind.node().is_some()));
        self.monsters = monsters_of(&self.world);
        self.player.health = self.player.health.min(self.max_health());
    }

    /// One tick of everything: the world's, then the player's.
    pub fn tick(&mut self) {
        self.tick_world();
        self.tick_player();
    }

    /// The world's tick: spent nodes come back, monsters go about and
    /// fight, health mends.
    pub fn tick_world(&mut self) {
        self.ticks += 1;
        let now = self.ticks;
        self.worked.retain(|_, w| w.back_at.is_none_or(|at| at > now));
        self.monsters_act();
        self.recover();
    }

    /// The player's tick: the next step of their way, then what they
    /// came to do, then a swing at it.
    pub fn tick_player(&mut self) {
        self.player.swung = false;
        self.step();
        self.engage(true);
        self.arrive();
        self.work();
        self.fight();
    }

    /// Having got where they were going, start on what they went for.
    fn arrive(&mut self) {
        let Some(Intent::Use(at)) = self.player.intent else {
            return;
        };
        if !self.player.path.is_empty() {
            return;
        }
        self.player.intent = None;
        let (dx, dy) = (at.0 - self.player.tile.0, at.1 - self.player.tile.1);
        if dx.abs() > 1 || dy.abs() > 1 {
            return;
        }
        if self.grave.as_ref().is_some_and(|g| g.tile == at) {
            return self.open_grave();
        }
        let Some(node) = self.node_at(at).filter(|_| (dx, dy) != (0, 0)) else {
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
        self.player.swung = true;
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
        self.gain(node.skill, node.xp);
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
        self.player.from = self.player.tile;
        let Some(&next) = self.player.path.front() else {
            self.player.goal = None;
            return;
        };
        let step = (next.0 - self.player.tile.0, next.1 - self.player.tile.1);
        let free = path::can_step(&self.world, self.player.tile, step) && !self.monster_on(next);
        let player = &mut self.player;
        if free {
            player.path.pop_front();
            player.tile = next;
            player.steps += 1;
            if player.path.is_empty() {
                player.goal = None;
            }
        } else if let Some(goal) = player.goal {
            // Something is in the way that was not (the map changed, or
            // a monster walked into it): find another way,
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
