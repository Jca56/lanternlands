//! Potions: brewed at a cauldron, drunk from the bag. One gives back
//! health at once; the others last a while. After drinking one there is
//! a short wait before the next of its kind. What a potion is doing is
//! counted in the world's ticks, is not saved, and ends with death.

use crate::items::Item;
use crate::player::{Event, Sim};

/// World ticks from drinking one to being able to drink another of the
/// same kind.
pub const DRINK_TICKS: u64 = 15;
/// What a healing potion gives back.
pub const HEALING: u32 = 15;
/// What a strength potion adds to the most a blow does, a stoneskin
/// potion to armour, and how long each lasts.
pub const STRENGTH: u32 = 2;
pub const STONESKIN: u8 = 4;
pub const LASTING_TICKS: u64 = 150;
/// A regeneration potion gives one health every this many ticks, for
/// this long.
pub const REGENERATION_EVERY: u64 = 2;
pub const REGENERATION_TICKS: u64 = 75;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Potion {
    Healing,
    Strength,
    Stoneskin,
    Regeneration,
}

impl Potion {
    pub const ALL: [Potion; 4] = [Potion::Healing, Potion::Strength, Potion::Stoneskin, Potion::Regeneration];

    /// The item it is carried as.
    pub const fn item(self) -> Item {
        match self {
            Potion::Healing => Item::HealingPotion,
            Potion::Strength => Item::StrengthPotion,
            Potion::Stoneskin => Item::StoneskinPotion,
            Potion::Regeneration => Item::RegenerationPotion,
        }
    }

    /// How long what it does lasts, in world ticks: none for one that
    /// does it all at once.
    pub const fn lasts(self) -> u64 {
        match self {
            Potion::Healing => 0,
            Potion::Strength | Potion::Stoneskin => LASTING_TICKS,
            Potion::Regeneration => REGENERATION_TICKS,
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

impl Item {
    /// The potion it is, if it is drunk.
    pub const fn potion(self) -> Option<Potion> {
        match self {
            Item::HealingPotion => Some(Potion::Healing),
            Item::StrengthPotion => Some(Potion::Strength),
            Item::StoneskinPotion => Some(Potion::Stoneskin),
            Item::RegenerationPotion => Some(Potion::Regeneration),
            _ => None,
        }
    }
}

/// What the potions the player has drunk are doing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tonics {
    /// The world tick each kind can be drunk again.
    ready_at: [u64; Potion::ALL.len()],
    /// The world tick what each kind does wears off.
    until: [u64; Potion::ALL.len()],
}

impl Tonics {
    /// Whether any is still working, or still waiting to be drunk again.
    pub(crate) fn running(&self, now: u64) -> bool {
        self.ready_at.iter().chain(&self.until).any(|&at| at > now)
    }

    /// What they do stops; the waits stay.
    pub(crate) fn wear_off(&mut self) {
        self.until = [0; Potion::ALL.len()];
    }
}

impl Sim {
    /// World ticks until a kind of potion can be drunk again: 0 when it
    /// can.
    pub fn drink_wait(&self, potion: Potion) -> u64 {
        self.tonics.ready_at[potion.index()].saturating_sub(self.ticks)
    }

    /// World ticks of a potion's doing left: 0 when it is doing nothing.
    pub fn potion_left(&self, potion: Potion) -> u64 {
        self.tonics.until[potion.index()].saturating_sub(self.ticks)
    }

    /// Drink the potion in a slot of the bag. Nothing happens for a
    /// slot that holds none, for a kind still waiting, or for a healing
    /// potion when there is nothing to heal.
    pub fn drink(&mut self, bag_slot: usize) {
        let Some(potion) = self.inventory.slots().get(bag_slot).copied().flatten().and_then(|held| held.item.potion()) else {
            return;
        };
        if self.drink_wait(potion) > 0 {
            return;
        }
        if potion == Potion::Healing && self.player.health >= self.max_health() {
            return;
        }
        self.inventory.set_slot(bag_slot, None);
        self.tonics.ready_at[potion.index()] = self.ticks + DRINK_TICKS;
        self.tonics.until[potion.index()] = self.ticks + potion.lasts();
        let healed = match potion {
            Potion::Healing => {
                let healed = HEALING.min(self.max_health() - self.player.health);
                self.player.health += healed;
                healed
            }
            _ => 0,
        };
        self.events.push(Event::Drank { potion, healed });
    }

    /// Use what is in a slot of the bag: gear is put on, a potion drunk.
    pub fn use_slot(&mut self, bag_slot: usize) {
        match self.inventory.slots().get(bag_slot).copied().flatten() {
            Some(held) if held.item.potion().is_some() => self.drink(bag_slot),
            Some(_) => self.equip(bag_slot),
            None => {}
        }
    }

    /// The world's tick for a regeneration potion: health back, a little
    /// at a time, in a fight or out of one.
    pub(crate) fn regenerate(&mut self) {
        if self.potion_left(Potion::Regeneration) > 0 && self.ticks.is_multiple_of(REGENERATION_EVERY) {
            self.player.health = (self.player.health + 1).min(self.max_health());
        }
    }
}

#[cfg(test)]
#[path = "potions_tests.rs"]
mod tests;
