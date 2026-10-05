//! The bank: what the player owns and does not carry.
//!
//! A bank chest is a prop on the map. The player walks up to one and is
//! then at the bank until they walk off; at it, things go between the
//! bag and the bank. There is one bank, whichever chest it is reached
//! through. In it everything stacks, gear and potions too, so each kind
//! of thing takes one slot however many there are; and a kind keeps its
//! slot for as long as there is any of it, so nothing shuffles about.
//! What is in the bank is safe: dying does not touch it.

use crate::items::{Item, Stack};
use crate::player::{Event, Sim};
use crate::world::PropKind;

/// How many kinds of thing the bank holds.
pub const SLOTS: usize = 48;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bank {
    slots: [Option<Stack>; SLOTS],
}

impl Default for Bank {
    fn default() -> Bank {
        Bank { slots: [None; SLOTS] }
    }
}

impl Bank {
    pub fn slots(&self) -> &[Option<Stack>; SLOTS] {
        &self.slots
    }

    pub fn count(&self, item: Item) -> u32 {
        self.slots.iter().flatten().find(|s| s.item == item).map_or(0, |s| s.count)
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(Option::is_none)
    }

    /// How many kinds of thing it holds: the slots taken.
    pub fn kinds(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    /// Put `count` of an item in: onto its stack, or into the first free
    /// slot when it is a new kind. Gives how many did not go in: all of
    /// them, when it is a new kind and every slot is taken.
    pub fn put(&mut self, item: Item, count: u32) -> u32 {
        let held = self.slots.iter().position(|s| s.is_some_and(|s| s.item == item));
        let Some(slot) = held.or_else(|| self.slots.iter().position(Option::is_none)) else {
            return count;
        };
        let before = self.slots[slot].map_or(0, |s| s.count);
        let room = (u32::MAX - before).min(count);
        if before + room > 0 {
            self.slots[slot] = Some(Stack { item, count: before + room });
        }
        count - room
    }

    /// Take up to `count` out of a slot. Gives how many came out.
    pub fn take(&mut self, slot: usize, count: u32) -> u32 {
        let Some(stack) = self.slots.get(slot).copied().flatten() else {
            return 0;
        };
        let taken = stack.count.min(count);
        self.slots[slot] = Some(Stack { count: stack.count - taken, ..stack }).filter(|s| s.count > 0);
        taken
    }

    /// Make a slot hold exactly `stack`: for a bank read back from its
    /// save.
    pub fn set_slot(&mut self, slot: usize, stack: Option<Stack>) {
        if let Some(held) = self.slots.get_mut(slot) {
            *held = stack.filter(|s| s.count > 0);
        }
    }
}

impl PropKind {
    /// Whether the bank is reached through it.
    pub const fn is_bank(self) -> bool {
        matches!(self, PropKind::BankChest)
    }
}

impl Sim {
    /// Whether what stands on a tile is a bank chest.
    pub fn bank_at(&self, tile: (i32, i32)) -> bool {
        self.world.prop_at(tile.0, tile.1).is_some_and(|p| p.kind.is_bank())
    }

    /// Whether the player has walked up to a bank chest and not yet left.
    pub fn banking(&self) -> bool {
        self.player.bank.is_some()
    }

    /// Step back from the bank.
    pub fn leave_bank(&mut self) {
        self.player.bank = None;
    }

    /// Put up to `count` of what a slot of the bag holds into the bank:
    /// from that slot first, then from wherever else in the bag there is
    /// more of it. Gives how many went. Away from the bank, none do.
    pub fn deposit(&mut self, slot: usize, count: u32) -> u32 {
        let Some(stack) = self.inventory.slots().get(slot).copied().flatten().filter(|_| self.banking()) else {
            return 0;
        };
        let wanted = count.min(self.inventory.count(stack.item));
        let moved = wanted - self.bank.put(stack.item, wanted);
        let here = stack.count.min(moved);
        self.inventory.set_slot(slot, Some(Stack { count: stack.count - here, ..stack }).filter(|s| s.count > 0));
        self.inventory.take(stack.item, moved - here);
        if moved < wanted {
            self.events.push(Event::BankFull);
        }
        moved
    }

    /// Put everything in the bag into the bank, as far as it goes in.
    pub fn deposit_all(&mut self) {
        let mut full = false;
        for slot in 0..self.inventory.slots().len() {
            let before = self.events.len();
            self.deposit(slot, u32::MAX);
            // Said once, however many things there was no slot for.
            full |= self.events.len() > before;
            self.events.truncate(before);
        }
        if full {
            self.events.push(Event::BankFull);
        }
    }

    /// Take up to `count` of what a slot of the bank holds into the bag,
    /// as many as there is room for. Gives how many came. Away from the
    /// bank, none do.
    pub fn withdraw(&mut self, slot: usize, count: u32) -> u32 {
        let Some(stack) = self.bank.slots().get(slot).copied().flatten().filter(|_| self.banking()) else {
            return 0;
        };
        let wanted = count.min(stack.count);
        let moved = wanted - self.inventory.add(stack.item, wanted);
        self.bank.take(slot, moved);
        if moved < wanted {
            self.events.push(Event::InventoryFull);
        }
        moved
    }
}

#[cfg(test)]
#[path = "bank_tests.rs"]
mod tests;
