//! Props that give something to whoever works at them: a rock gives
//! ore. A node is worked in swings, each with a chance of getting one;
//! after it has given what it holds it is spent, and comes back later.

use crate::items::Item;
use crate::player::{Activity, Event, Sim, Worked};
use crate::skills::Skill;
use crate::world::PropKind;

/// No swing is ever surer than this.
pub const BEST_CHANCE: f64 = 0.95;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub skill: Skill,
    /// The level it takes to work it at all.
    pub level: u8,
    pub item: Item,
    /// Experience for each one got.
    pub xp: u32,
    /// How many it gives before it is spent.
    pub yields: u8,
    /// Ticks one swing takes.
    pub swing_ticks: u32,
    /// The chance a swing gets one at the level it takes, and how much
    /// every level past that adds.
    pub chance: f64,
    pub chance_per_level: f64,
    /// Ticks from spent to whole again.
    pub respawn_ticks: u64,
}

impl Node {
    /// The chance a swing gets one, for someone of `level`. A node that
    /// is sure to begin with stays sure.
    pub fn chance_at(&self, level: u8) -> f64 {
        if self.chance >= 1.0 {
            return 1.0;
        }
        let past = level.saturating_sub(self.level) as f64;
        (self.chance + self.chance_per_level * past).min(BEST_CHANCE)
    }
}

impl PropKind {
    /// What working at it gives, if it is a node.
    pub const fn node(self) -> Option<Node> {
        match self {
            // A copper rock.
            PropKind::Rock => Some(Node { skill: Skill::Mining, level: 1, item: Item::CopperOre, xp: 12, yields: 3, swing_ticks: 3, chance: 0.55, chance_per_level: 0.03, respawn_ticks: 75 }),
            // A tin rock: the same to mine, for the other half of bronze.
            PropKind::TinRock => Some(Node { skill: Skill::Mining, level: 1, item: Item::TinOre, xp: 12, yields: 3, swing_ticks: 3, chance: 0.55, chance_per_level: 0.03, respawn_ticks: 75 }),
            // Herbs: a quick pick that never fails, two to a plant.
            PropKind::HerbPurple => Some(Node { skill: Skill::Herbalism, level: 1, item: Item::PurpleHerb, xp: 10, yields: 2, swing_ticks: 2, chance: 1.0, chance_per_level: 0.0, respawn_ticks: 75 }),
            PropKind::HerbCyan => Some(Node { skill: Skill::Herbalism, level: 3, item: Item::CyanHerb, xp: 14, yields: 2, swing_ticks: 2, chance: 1.0, chance_per_level: 0.0, respawn_ticks: 75 }),
            PropKind::Tree | PropKind::Furnace | PropKind::Anvil | PropKind::Cauldron | PropKind::BankChest => None,
            PropKind::Crates | PropKind::Barrel | PropKind::Signpost | PropKind::Well | PropKind::Fence => None,
        }
    }
}

impl Sim {
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

    /// How many ticks a swing at a node takes the player: fewer with
    /// the right tool in the bag, and never less than one.
    pub fn swing_ticks(&self, node: &Node) -> u32 {
        let quicker = match node.skill {
            Skill::Mining => self.inventory.slots().iter().flatten().map(|stack| stack.item.mining_speed()).max().unwrap_or(0),
            _ => 0,
        };
        node.swing_ticks.saturating_sub(quicker).max(1)
    }

    /// A tick of work: when a swing is done, roll for what it got.
    pub(crate) fn work(&mut self) {
        let Some(Activity::Working { at, swing }) = self.player.activity else {
            return;
        };
        let Some(node) = self.node_at(at).filter(|_| !self.is_spent(at)) else {
            // It went from under them: edited away, or spent.
            self.player.activity = None;
            return;
        };
        if swing + 1 < self.swing_ticks(&node) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rock_is_a_node_and_scenery_is_not() {
        let rock = PropKind::Rock.node().expect("a node");
        assert_eq!((rock.skill, rock.item, rock.level), (Skill::Mining, Item::CopperOre, 1));
        assert!(rock.yields > 0 && rock.swing_ticks > 0 && rock.respawn_ticks > 0);
        assert_eq!(PropKind::Tree.node(), None);
    }

    #[test]
    fn the_chance_climbs_with_level_and_stops_short_of_certain() {
        let rock = PropKind::Rock.node().unwrap();
        assert_eq!(rock.chance_at(1), 0.55);
        assert!((rock.chance_at(6) - 0.70).abs() < 1e-12);
        assert!((1..50).all(|l| rock.chance_at(l + 1) >= rock.chance_at(l)));
        assert_eq!(rock.chance_at(50), BEST_CHANCE);
        // Under the level it takes is no worse than at it; the level is
        // checked before a swing is ever taken.
        let hard = Node { level: 20, ..rock };
        assert_eq!(hard.chance_at(5), hard.chance);
    }
}
