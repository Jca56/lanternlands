//! Things that can be carried, and the bag they are carried in.

/// How many slots the inventory has.
pub const SLOTS: usize = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    CopperOre,
}

impl Item {
    pub const ALL: [Item; 1] = [Item::CopperOre];

    /// What it is called in files (its picture's name). Never change one.
    pub const fn name(self) -> &'static str {
        match self {
            Item::CopperOre => "copper_ore",
        }
    }

    /// Its name as the player reads it.
    pub const fn label(self) -> &'static str {
        match self {
            Item::CopperOre => "Copper ore",
        }
    }

    /// Whether any number of it share a slot. Materials do; gear will not.
    pub const fn stacks(self) -> bool {
        match self {
            Item::CopperOre => true,
        }
    }
}

/// What one slot holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stack {
    pub item: Item,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    slots: [Option<Stack>; SLOTS],
}

impl Default for Inventory {
    fn default() -> Inventory {
        Inventory { slots: [None; SLOTS] }
    }
}

impl Inventory {
    pub fn slots(&self) -> &[Option<Stack>; SLOTS] {
        &self.slots
    }

    /// How many of an item are carried, over every slot.
    pub fn count(&self, item: Item) -> u32 {
        self.slots.iter().flatten().filter(|s| s.item == item).map(|s| s.count).sum()
    }

    /// Whether one more of an item would fit.
    pub fn has_room_for(&self, item: Item) -> bool {
        self.slots.iter().any(|slot| match slot {
            None => true,
            Some(stack) => stack.item == item && item.stacks() && stack.count < u32::MAX,
        })
    }

    /// Put `count` of an item in: onto its stack if it has one, else
    /// into the first empty slot (one slot each for what does not
    /// stack). Returns how many did not fit.
    pub fn add(&mut self, item: Item, mut count: u32) -> u32 {
        while count > 0 {
            let stack = self.slots.iter_mut().flatten().find(|s| s.item == item && item.stacks() && s.count < u32::MAX);
            match stack {
                Some(stack) => {
                    let room = (u32::MAX - stack.count).min(count);
                    stack.count += room;
                    count -= room;
                }
                None => {
                    let Some(empty) = self.slots.iter_mut().find(|s| s.is_none()) else {
                        break;
                    };
                    let put = if item.stacks() { count } else { 1 };
                    *empty = Some(Stack { item, count: put });
                    count -= put;
                }
            }
        }
        count
    }

    /// Take `count` of an item out, from the last slots first. `false`,
    /// and nothing is taken, when fewer are carried.
    pub fn take(&mut self, item: Item, mut count: u32) -> bool {
        if self.count(item) < count {
            return false;
        }
        for slot in self.slots.iter_mut().rev() {
            if let Some(stack) = slot.filter(|s| s.item == item) {
                let taken = stack.count.min(count);
                count -= taken;
                *slot = Some(Stack { count: stack.count - taken, ..stack }).filter(|s| s.count > 0);
            }
        }
        true
    }

    /// Trade what two slots hold: how the player tidies the bag.
    pub fn swap(&mut self, a: usize, b: usize) {
        if a < SLOTS && b < SLOTS {
            self.slots.swap(a, b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_stack_in_one_slot() {
        let mut bag = Inventory::default();
        assert_eq!(bag.add(Item::CopperOre, 3), 0);
        assert_eq!(bag.add(Item::CopperOre, 4), 0);
        assert_eq!(bag.slots()[0], Some(Stack { item: Item::CopperOre, count: 7 }));
        assert!(bag.slots()[1..].iter().all(Option::is_none));
        assert_eq!(bag.count(Item::CopperOre), 7);
    }

    #[test]
    fn a_full_bag_still_takes_what_stacks_and_says_what_did_not_fit() {
        // Every slot holding something that is full to the brim.
        let mut bag = Inventory { slots: [Some(Stack { item: Item::CopperOre, count: u32::MAX }); SLOTS] };
        assert!(!bag.has_room_for(Item::CopperOre));
        assert_eq!(bag.add(Item::CopperOre, 5), 5);
        // One stack with room, and it all goes there.
        bag.slots[9] = Some(Stack { item: Item::CopperOre, count: 10 });
        assert!(bag.has_room_for(Item::CopperOre));
        assert_eq!(bag.add(Item::CopperOre, 5), 0);
        assert_eq!(bag.slots()[9].unwrap().count, 15);
    }

    #[test]
    fn an_empty_bag_has_thirty_slots_of_room() {
        let bag = Inventory::default();
        assert_eq!(bag.slots().len(), 30);
        assert!(bag.has_room_for(Item::CopperOre));
        assert_eq!(bag.count(Item::CopperOre), 0);
    }

    #[test]
    fn taking_is_all_or_nothing_and_empties_a_slot_at_zero() {
        let mut bag = Inventory::default();
        bag.add(Item::CopperOre, 5);
        assert!(!bag.take(Item::CopperOre, 6));
        assert_eq!(bag.count(Item::CopperOre), 5);
        assert!(bag.take(Item::CopperOre, 2));
        assert_eq!(bag.count(Item::CopperOre), 3);
        assert!(bag.take(Item::CopperOre, 3));
        assert!(bag.slots().iter().all(Option::is_none));
    }

    #[test]
    fn swapping_trades_two_slots_and_ignores_slots_that_are_not_there() {
        let mut bag = Inventory::default();
        bag.add(Item::CopperOre, 2);
        bag.swap(0, 17);
        assert_eq!((bag.slots()[0], bag.slots()[17].map(|s| s.count)), (None, Some(2)));
        bag.swap(17, SLOTS);
        assert_eq!(bag.slots()[17].map(|s| s.count), Some(2));
    }
}
