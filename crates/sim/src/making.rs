//! Making things at stations: ore smelted into bars at a furnace, bars
//! hammered into gear at an anvil.
//!
//! The player walks up to a station and is then at it until they walk
//! off. At it, they can be set to make a number of something: one is
//! made every few ticks, taking what it is made from out of the bag,
//! until that many are made, what it takes runs out, or there is no
//! room for another.

use crate::items::Item;
use crate::player::{Activity, Event, Sim};
use crate::skills::Skill;
use crate::world::PropKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Station {
    Furnace,
    Anvil,
}

impl Station {
    pub const ALL: [Station; 2] = [Station::Furnace, Station::Anvil];

    pub const fn label(self) -> &'static str {
        match self {
            Station::Furnace => "Furnace",
            Station::Anvil => "Anvil",
        }
    }
}

/// Something that can be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub station: Station,
    pub makes: Item,
    /// What one takes, and how many of each.
    pub from: &'static [(Item, u32)],
    /// The Smithing level it takes.
    pub level: u8,
    /// Experience for each one made.
    pub xp: u32,
    /// Ticks one takes to make.
    pub ticks: u32,
}

const fn forged(makes: Item, bars: &'static [(Item, u32)], level: u8) -> Recipe {
    // Fifteen experience a bar, four ticks whatever it is.
    Recipe { station: Station::Anvil, makes, from: bars, level, xp: 15 * bars[0].1, ticks: 4 }
}

/// Everything that can be made, in the order the panels list it.
pub const RECIPES: [Recipe; 9] = [
    Recipe { station: Station::Furnace, makes: Item::BronzeBar, from: &[(Item::CopperOre, 1), (Item::TinOre, 1)], level: 1, xp: 10, ticks: 3 },
    forged(Item::BronzePickaxe, &[(Item::BronzeBar, 2)], 1),
    forged(Item::BronzeSword, &[(Item::BronzeBar, 2)], 2),
    forged(Item::BronzeBoots, &[(Item::BronzeBar, 1)], 2),
    forged(Item::BronzeGauntlets, &[(Item::BronzeBar, 1)], 3),
    forged(Item::BronzeHelmet, &[(Item::BronzeBar, 2)], 3),
    forged(Item::BronzeShield, &[(Item::BronzeBar, 3)], 4),
    forged(Item::BronzeGreaves, &[(Item::BronzeBar, 3)], 4),
    forged(Item::BronzeChestplate, &[(Item::BronzeBar, 5)], 5),
];

impl PropKind {
    /// The station it is, if things are made at it.
    pub const fn station(self) -> Option<Station> {
        match self {
            PropKind::Furnace => Some(Station::Furnace),
            PropKind::Anvil => Some(Station::Anvil),
            _ => None,
        }
    }
}

impl Sim {
    /// The station on a tile, if what stands there is one.
    pub fn station_at(&self, tile: (i32, i32)) -> Option<Station> {
        self.world.prop_at(tile.0, tile.1)?.kind.station()
    }

    /// The station the player has walked up to and not yet left.
    pub fn station(&self) -> Option<Station> {
        self.player.station.map(|(station, _)| station)
    }

    /// Step back from the station: whatever was being made is dropped.
    pub fn leave_station(&mut self) {
        self.player.station = None;
        if matches!(self.player.activity, Some(Activity::Making { .. })) {
            self.player.activity = None;
        }
    }

    /// How many of a recipe what is carried would make, whatever the
    /// player's level.
    pub fn makeable(&self, recipe: &Recipe) -> u32 {
        recipe.from.iter().map(|&(item, each)| self.inventory.count(item) / each.max(1)).min().unwrap_or(0)
    }

    /// Whether the player's Smithing is up to a recipe.
    pub fn can_smith(&self, recipe: &Recipe) -> bool {
        self.skills.level(Skill::Smithing) >= recipe.level
    }

    /// The recipe being made, how many are still to make, and how far
    /// through the one in hand they are, in ticks.
    pub fn making(&self) -> Option<(&'static Recipe, u32, u32)> {
        match self.player.activity {
            Some(Activity::Making { recipe, left, tick, .. }) => Some((&RECIPES[recipe], left, tick)),
            _ => None,
        }
    }

    /// Set the player to make `count` of `RECIPES[recipe]` (or as many
    /// as what they carry allows) at the station they are at. Asked for
    /// at the wrong station, or anywhere else, nothing happens.
    pub fn make(&mut self, recipe: usize, count: u32) {
        let (Some(wanted), Some((station, at))) = (RECIPES.get(recipe), self.player.station) else {
            return;
        };
        if wanted.station != station {
            return;
        }
        if !self.can_smith(wanted) {
            self.events.push(Event::TooHard { skill: Skill::Smithing, level: wanted.level });
            return;
        }
        let left = count.min(self.makeable(wanted));
        if left > 0 {
            self.player.activity = Some(Activity::Making { at, recipe, left, tick: 0 });
        }
    }

    /// A tick of making: when one is done, what it takes leaves the
    /// bag and it goes in.
    pub(crate) fn craft(&mut self) {
        let Some(Activity::Making { at, recipe, left, tick }) = self.player.activity else {
            return;
        };
        let wanted = &RECIPES[recipe];
        if self.station_at(at) != Some(wanted.station) {
            // The station went from under them: edited away.
            self.player.activity = None;
            self.player.station = None;
            return;
        }
        if tick + 1 < wanted.ticks {
            self.player.activity = Some(Activity::Making { at, recipe, left, tick: tick + 1 });
            return;
        }
        self.player.activity = None;
        if self.makeable(wanted) == 0 {
            return;
        }
        // What it takes comes out first: that may be what makes room.
        let before = self.inventory.clone();
        for &(item, each) in wanted.from {
            self.inventory.take(item, each);
        }
        if self.inventory.add(wanted.makes, 1) > 0 {
            self.inventory = before;
            self.events.push(Event::InventoryFull);
            return;
        }
        self.player.swung = true;
        self.events.push(Event::Got { item: wanted.makes, count: 1 });
        self.gain(Skill::Smithing, wanted.xp);
        if left > 1 && self.makeable(wanted) > 0 {
            self.player.activity = Some(Activity::Making { at, recipe, left: left - 1, tick: 0 });
        }
    }
}

#[cfg(test)]
#[path = "making_tests.rs"]
mod tests;
