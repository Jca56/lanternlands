//! What roams the world and fights: the kinds of monster, and a
//! monster as the simulation keeps it.

use crate::items::Item;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MonsterKind {
    Goblin,
}

/// What a kind of monster is like.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub level: u8,
    pub health: u32,
    /// What its hits are rolled with, and what hits on it are rolled against.
    pub attack: u8,
    pub defence: u8,
    /// The most one of its hits does.
    pub max_hit: u32,
    /// Ticks from one of its attacks to the next.
    pub attack_ticks: u32,
    /// How near the player must come for it to go for them, in tiles.
    pub sight: i32,
    /// How far from its home it follows them before giving up.
    pub leash: i32,
    /// How far from its home it wanders when left alone.
    pub roam: i32,
    /// Ticks from its death to its return.
    pub respawn_ticks: u64,
    /// What it may leave behind, and how often.
    pub drop: Option<(Item, f64)>,
}

impl MonsterKind {
    pub const ALL: [MonsterKind; 1] = [MonsterKind::Goblin];

    /// What it is called in a map file. Never change one.
    pub const fn name(self) -> &'static str {
        match self {
            MonsterKind::Goblin => "goblin",
        }
    }

    pub fn from_name(name: &str) -> Option<MonsterKind> {
        MonsterKind::ALL.into_iter().find(|k| k.name() == name)
    }

    /// Its name as the player reads it.
    pub const fn label(self) -> &'static str {
        match self {
            MonsterKind::Goblin => "Goblin",
        }
    }

    pub const fn stats(self) -> Stats {
        match self {
            MonsterKind::Goblin => Stats { level: 2, health: 12, attack: 1, defence: 1, max_hit: 2, attack_ticks: 4, sight: 4, leash: 8, roam: 3, respawn_ticks: 60, drop: Some((Item::GoblinTooth, 0.5)) },
        }
    }
}

/// One monster in the world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Monster {
    pub kind: MonsterKind,
    /// The tile it lives on: where it appears, and what it stays near.
    pub home: (i32, i32),
    pub tile: (i32, i32),
    /// The tile it was on before the last tick: the same, when it did
    /// not move.
    pub from: (i32, i32),
    pub health: u32,
    /// It is after the player.
    pub angry: bool,
    /// The tick it last swung at the player.
    pub swung_at: Option<u64>,
    /// The tick it can move and strike again, when it has been stunned.
    pub stunned_until: u64,
    /// Ticks until it may attack again.
    pub(crate) cooldown: u32,
    /// The tick it comes back, while it is dead.
    pub(crate) back_at: Option<u64>,
    /// The tick it next thinks of wandering.
    pub(crate) wander_at: u64,
}

impl Monster {
    pub fn new(kind: MonsterKind, home: (i32, i32)) -> Monster {
        Monster { kind, home, tile: home, from: home, health: kind.stats().health, angry: false, swung_at: None, stunned_until: 0, cooldown: 0, back_at: None, wander_at: 0 }
    }

    pub fn alive(&self) -> bool {
        self.back_at.is_none()
    }

    /// Whether the last tick moved it.
    pub fn moved(&self) -> bool {
        self.tile != self.from
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for kind in MonsterKind::ALL {
            assert_eq!(MonsterKind::from_name(kind.name()), Some(kind));
        }
        assert_eq!(MonsterKind::from_name("dragon"), None);
    }

    #[test]
    fn a_goblin_makes_sense() {
        let goblin = MonsterKind::Goblin.stats();
        assert!(goblin.health > 0 && goblin.max_hit > 0 && goblin.attack_ticks > 0);
        // It can see no further than it will chase, and wanders nearer still.
        assert!(goblin.roam < goblin.sight && goblin.sight < goblin.leash);
        let fresh = Monster::new(MonsterKind::Goblin, (4, 7));
        assert!(fresh.alive() && !fresh.moved() && !fresh.angry);
        assert_eq!((fresh.tile, fresh.health), ((4, 7), goblin.health));
    }
}
