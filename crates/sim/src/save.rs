//! A character on disk: who they are, where they stand, what they
//! carry and know, and which nodes they left spent.
//!
//! A save file, little-endian throughout, in sections like a chunk
//! file's:
//!
//! ```text
//! "SAVE"  version u16  sections u8
//! "CHAR"  name (length u8 + UTF-8), seconds played u64
//! "PLYR"  x i32, y i32, steps u64, ticks u64
//! "SKIL"  names (count u8, each length u8 + UTF-8), then experience
//!         u32 for each, in that order
//! "BAG "  slots u16, names likewise, then for each slot: name index
//!         u8 (255 = empty), count u32
//! "NODE"  count u32, then each: x i32, y i32, left u8, ticks until
//!         whole again u32 (all ones = not coming back by itself)
//! ```
//!
//! Skills and items are written by name, so new ones can be added and
//! the order changed without breaking a save. A section this build
//! does not know is skipped; a name it does not know is an error.

use crate::items::{Inventory, Item, SLOTS, Stack};
use crate::player::{Sim, Worked};
use crate::skills::{Skill, Skills};
use crate::wire::{Reader, name_table, write_names, write_sections, write_text};
use crate::world::{VOID, World};

const MAGIC: [u8; 4] = *b"SAVE";
const VERSION: u16 = 1;
const CHARACTER: [u8; 4] = *b"CHAR";
const PLAYER: [u8; 4] = *b"PLYR";
const SKILLS: [u8; 4] = *b"SKIL";
const BAG: [u8; 4] = *b"BAG ";
const NODES: [u8; 4] = *b"NODE";
const NEVER: u32 = u32::MAX;

/// Who a save belongs to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Character {
    pub name: String,
    /// Seconds spent playing them.
    pub played: u64,
}

/// A node that was not whole when the game was saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedNode {
    pub at: (i32, i32),
    pub left: u8,
    /// Ticks until it is whole again; `None` while it still holds some.
    pub wait: Option<u32>,
}

/// Everything a save file holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub character: Character,
    pub tile: (i32, i32),
    pub steps: u64,
    pub ticks: u64,
    pub skills: Skills,
    pub inventory: Inventory,
    pub nodes: Vec<SavedNode>,
}

impl Saved {
    pub fn encode(&self) -> Vec<u8> {
        let mut character = Vec::new();
        write_text(&mut character, &self.character.name);
        character.extend_from_slice(&self.character.played.to_le_bytes());

        let mut player = Vec::new();
        player.extend_from_slice(&self.tile.0.to_le_bytes());
        player.extend_from_slice(&self.tile.1.to_le_bytes());
        player.extend_from_slice(&self.steps.to_le_bytes());
        player.extend_from_slice(&self.ticks.to_le_bytes());

        let mut skills = Vec::new();
        write_names(&mut skills, &Skill::ALL.map(Skill::name));
        for skill in Skill::ALL {
            skills.extend_from_slice(&self.skills.xp(skill).to_le_bytes());
        }

        let mut bag = Vec::new();
        let slots = self.inventory.slots();
        let (names, indices) = name_table(slots.iter().map(|slot| slot.map(|s| s.item.name())));
        bag.extend_from_slice(&(slots.len() as u16).to_le_bytes());
        write_names(&mut bag, &names);
        for (slot, index) in slots.iter().zip(indices) {
            bag.push(index);
            bag.extend_from_slice(&slot.map_or(0, |s| s.count).to_le_bytes());
        }

        let mut nodes = Vec::new();
        // In a fixed order, so the same game is always the same bytes.
        let mut worked = self.nodes.clone();
        worked.sort_by_key(|n| (n.at.1, n.at.0));
        nodes.extend_from_slice(&(worked.len() as u32).to_le_bytes());
        for node in worked {
            nodes.extend_from_slice(&node.at.0.to_le_bytes());
            nodes.extend_from_slice(&node.at.1.to_le_bytes());
            nodes.push(node.left);
            nodes.extend_from_slice(&node.wait.unwrap_or(NEVER).to_le_bytes());
        }

        let sections = [(CHARACTER, character), (PLAYER, player), (SKILLS, skills), (BAG, bag), (NODES, nodes)];
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.push(sections.len() as u8);
        write_sections(&mut out, &sections);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Saved, String> {
        let mut file = Reader { bytes };
        if file.tag()? != MAGIC {
            return Err("not a save file".to_owned());
        }
        let version = file.u16()?;
        if version != VERSION {
            return Err(format!("it is version {version}; this build reads version {VERSION}"));
        }
        let mut saved = Saved { character: Character::default(), tile: (0, 0), steps: 0, ticks: 0, skills: Skills::default(), inventory: Inventory::default(), nodes: Vec::new() };
        let mut seen: Vec<[u8; 4]> = Vec::new();
        for _ in 0..file.u8()? {
            let tag = file.tag()?;
            let length = file.u32()? as usize;
            let mut section = Reader { bytes: file.take(length)? };
            if seen.contains(&tag) {
                return Err(format!("it has two {:?} sections", String::from_utf8_lossy(&tag)));
            }
            seen.push(tag);
            match tag {
                CHARACTER => {
                    saved.character = Character { name: section.text("the name")?, played: section.u64()? };
                    section.done("the character")?;
                }
                PLAYER => {
                    saved.tile = (section.i32()?, section.i32()?);
                    (saved.steps, saved.ticks) = (section.u64()?, section.u64()?);
                    section.done("the player")?;
                }
                SKILLS => {
                    for skill in section.names("skill", Skill::from_name)? {
                        saved.skills.set_xp(skill, section.u32()?);
                    }
                    section.done("the skills")?;
                }
                BAG => {
                    let slots = section.u16()? as usize;
                    if slots > SLOTS {
                        return Err(format!("its bag has {slots} slots; this build's has {SLOTS}"));
                    }
                    let names = section.names("item", Item::from_name)?;
                    for slot in 0..slots {
                        let (index, count) = (section.u8()?, section.u32()?);
                        if index != VOID {
                            let item = *names.get(index as usize).ok_or(format!("slot {slot} holds item number {index} of {}", names.len()))?;
                            saved.inventory.set_slot(slot, Some(Stack { item, count }));
                        }
                    }
                    section.done("the bag")?;
                }
                NODES => {
                    for _ in 0..section.u32()? {
                        let at = (section.i32()?, section.i32()?);
                        let (left, wait) = (section.u8()?, section.u32()?);
                        saved.nodes.push(SavedNode { at, left, wait: Some(wait).filter(|w| *w != NEVER) });
                    }
                    section.done("the nodes")?;
                }
                _ => {}
            }
        }
        file.done("the last section")?;
        for (tag, what) in [(CHARACTER, "character"), (PLAYER, "player")] {
            if !seen.contains(&tag) {
                return Err(format!("it has no {what} section"));
            }
        }
        Ok(saved)
    }
}

impl Sim {
    /// Everything about the game in play that a save keeps.
    pub fn snapshot(&self, character: &Character) -> Saved {
        let nodes = self.worked.iter().map(|(&at, w)| SavedNode { at, left: w.left, wait: w.back_at.map(|back| back.saturating_sub(self.ticks).min(NEVER as u64 - 1) as u32) }).collect();
        Saved { character: character.clone(), tile: self.player.tile, steps: self.player.steps, ticks: self.ticks, skills: self.skills.clone(), inventory: self.inventory.clone(), nodes }
    }

    /// The game a save holds, in `world` as it is now. The map may have
    /// changed since: the player stands as near where they were as they
    /// can, and nodes that are no longer there are forgotten.
    pub fn resume(world: World, saved: &Saved) -> Sim {
        let mut sim = Sim::new(world);
        sim.player.tile = saved.tile;
        sim.player.steps = saved.steps;
        sim.ticks = saved.ticks;
        sim.skills = saved.skills.clone();
        sim.inventory = saved.inventory.clone();
        for node in &saved.nodes {
            let back_at = node.wait.map(|wait| saved.ticks + wait as u64);
            sim.worked.insert(node.at, Worked { left: node.left, back_at });
        }
        sim.settle_player();
        sim
    }
}

#[cfg(test)]
#[path = "save_tests.rs"]
mod tests;
