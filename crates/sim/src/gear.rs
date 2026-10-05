//! What the player wears and holds: seven slots, each for one kind of
//! gear. Gear goes on from the bag and comes off into it; what is worn
//! takes no room there, makes blows surer and harder and the player
//! harder to hit, and stays on through death.

use crate::items::{Item, Stack};
use crate::player::{Event, Sim};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Weapon,
    Shield,
    Head,
    Body,
    Legs,
    Hands,
    Feet,
}

impl Slot {
    pub const ALL: [Slot; 7] = [Slot::Weapon, Slot::Shield, Slot::Head, Slot::Body, Slot::Legs, Slot::Hands, Slot::Feet];

    /// What it is called in a save. Never change one.
    pub const fn name(self) -> &'static str {
        match self {
            Slot::Weapon => "weapon",
            Slot::Shield => "shield",
            Slot::Head => "head",
            Slot::Body => "body",
            Slot::Legs => "legs",
            Slot::Hands => "hands",
            Slot::Feet => "feet",
        }
    }

    /// Its name as the player reads it.
    pub const fn label(self) -> &'static str {
        match self {
            Slot::Weapon => "Weapon",
            Slot::Shield => "Shield",
            Slot::Head => "Head",
            Slot::Body => "Body",
            Slot::Legs => "Legs",
            Slot::Hands => "Hands",
            Slot::Feet => "Feet",
        }
    }

    pub fn from_name(name: &str) -> Option<Slot> {
        Slot::ALL.into_iter().find(|s| s.name() == name)
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// What gear adds: to how sure the player's blows are and how hard, and
/// to how hard they are to hit. Accuracy and armour count as levels of
/// Melee and Defence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bonus {
    pub accuracy: u8,
    pub damage: u32,
    pub armour: u8,
}

/// What a kind of gear is made of: what it is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Metal {
    Bronze,
}

impl Item {
    /// The slot it is worn in, if it is worn at all.
    pub const fn slot(self) -> Option<Slot> {
        match self {
            Item::BronzeSword => Some(Slot::Weapon),
            Item::BronzeShield => Some(Slot::Shield),
            Item::BronzeHelmet => Some(Slot::Head),
            Item::BronzeChestplate => Some(Slot::Body),
            Item::BronzeGreaves => Some(Slot::Legs),
            Item::BronzeGauntlets => Some(Slot::Hands),
            Item::BronzeBoots => Some(Slot::Feet),
            _ => None,
        }
    }

    /// What wearing it adds.
    pub const fn bonus(self) -> Bonus {
        let (accuracy, damage, armour) = match self {
            Item::BronzeSword => (2, 2, 0),
            Item::BronzeShield => (0, 0, 2),
            Item::BronzeHelmet => (0, 0, 1),
            Item::BronzeChestplate => (0, 0, 3),
            Item::BronzeGreaves => (0, 0, 2),
            Item::BronzeGauntlets => (0, 0, 1),
            Item::BronzeBoots => (0, 0, 1),
            _ => (0, 0, 0),
        };
        Bonus { accuracy, damage, armour }
    }

    /// What it is made of, if it is worn.
    pub const fn metal(self) -> Option<Metal> {
        match self.slot() {
            Some(_) => Some(Metal::Bronze),
            None => None,
        }
    }
}

/// What is worn in each slot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gear {
    worn: [Option<Item>; Slot::ALL.len()],
}

impl Gear {
    pub fn worn(&self, slot: Slot) -> Option<Item> {
        self.worn[slot.index()]
    }

    /// Every slot with something in it.
    pub fn iter(&self) -> impl Iterator<Item = (Slot, Item)> + '_ {
        Slot::ALL.into_iter().filter_map(|slot| Some((slot, self.worn(slot)?)))
    }

    /// Put something in a slot, whatever was there; for gear read back
    /// from a save. What does not belong in the slot is not put there.
    pub fn set(&mut self, slot: Slot, item: Option<Item>) {
        if item.is_none_or(|item| item.slot() == Some(slot)) {
            self.worn[slot.index()] = item;
        }
    }

    /// All that is worn, added up.
    pub fn bonus(&self) -> Bonus {
        self.iter().fold(Bonus::default(), |sum, (_, item)| {
            let b = item.bonus();
            Bonus { accuracy: sum.accuracy + b.accuracy, damage: sum.damage + b.damage, armour: sum.armour + b.armour }
        })
    }
}

impl Sim {
    /// Put on what is in a slot of the bag. What was worn in its place
    /// goes into that slot of the bag, so there is always room. Nothing
    /// happens for a slot that is empty or holds what cannot be worn.
    pub fn equip(&mut self, bag_slot: usize) {
        let Some(held) = self.inventory.slots().get(bag_slot).copied().flatten() else {
            return;
        };
        let Some(slot) = held.item.slot() else {
            return;
        };
        let was = self.gear.worn(slot);
        self.gear.set(slot, Some(held.item));
        self.inventory.set_slot(bag_slot, was.map(|item| Stack { item, count: 1 }));
    }

    /// Take off what is worn in a slot, into the bag; with no room for
    /// it there, it stays on and the game says so.
    pub fn unequip(&mut self, slot: Slot) {
        let Some(item) = self.gear.worn(slot) else {
            return;
        };
        if self.inventory.add(item, 1) == 0 {
            self.gear.set(slot, None);
        } else {
            self.events.push(Event::InventoryFull);
        }
    }
}

impl Sim {
    /// Take off what is worn in a slot, into slot `into` of the bag,
    /// where the player dropped it: if that is empty it goes there, and
    /// if it holds something worn in the same slot the two trade
    /// places. Dropped on anything else, it goes wherever there is room.
    pub fn unequip_into(&mut self, slot: Slot, into: usize) {
        let Some(item) = self.gear.worn(slot) else {
            return;
        };
        match self.inventory.slots().get(into).copied() {
            Some(None) => {
                self.gear.set(slot, None);
                self.inventory.set_slot(into, Some(Stack { item, count: 1 }));
            }
            Some(Some(held)) if held.item.slot() == Some(slot) => self.equip(into),
            _ => self.unequip(slot),
        }
    }
}

#[cfg(test)]
#[path = "gear_tests.rs"]
mod tests;
