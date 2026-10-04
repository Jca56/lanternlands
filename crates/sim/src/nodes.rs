//! Props that give something to whoever works at them: a rock gives
//! ore. A node is worked in swings, each with a chance of getting one;
//! after it has given what it holds it is spent, and comes back later.

use crate::items::Item;
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
            PropKind::Tree | PropKind::Furnace | PropKind::Anvil | PropKind::Cauldron => None,
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
