//! Abilities: what the hotbar's buttons do.
//!
//! An ability asked for is used on the player's next tick, on top of
//! their own blows, and then cannot be used again until its time is up.
//! One aimed at a monster waits until the player is beside it. The
//! times are counted in the world's ticks, so they run down whatever
//! the player is doing.

use crate::combat::{MELEE_XP, hit_chance, in_reach};
use crate::player::{Event, Sim, Who};
use crate::skills::Skill;

/// How long a guard lasts, a kick stuns, and how many blows a flurry
/// is, one a tick.
pub const GUARD_TICKS: u64 = 10;
pub const STUN_TICKS: u64 = 6;
pub const FLURRY_BLOWS: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ability {
    /// One big blow that always lands.
    HeavyStrike,
    /// Three quick weak blows.
    Flurry,
    /// The target can do nothing for a while.
    Kick,
    /// Blows against the player do half for a while.
    Guard,
}

impl Ability {
    pub const ALL: [Ability; 4] = [Ability::HeavyStrike, Ability::Flurry, Ability::Kick, Ability::Guard];

    /// The name its picture's file goes by.
    pub fn name(self) -> &'static str {
        match self {
            Ability::HeavyStrike => "heavy_strike",
            Ability::Flurry => "flurry",
            Ability::Kick => "kick",
            Ability::Guard => "guard",
        }
    }

    /// The name the player sees.
    pub fn label(self) -> &'static str {
        match self {
            Ability::HeavyStrike => "Heavy Strike",
            Ability::Flurry => "Flurry",
            Ability::Kick => "Kick",
            Ability::Guard => "Guard",
        }
    }

    /// World ticks from one use to the next.
    pub fn cooldown(self) -> u64 {
        match self {
            Ability::HeavyStrike => 20,
            Ability::Flurry => 30,
            Ability::Kick => 38,
            Ability::Guard => 50,
        }
    }

    /// Whether it is aimed at a monster, and so wants one in reach.
    pub fn aimed(self) -> bool {
        self != Ability::Guard
    }

    fn index(self) -> usize {
        Ability::ALL.iter().position(|a| *a == self).expect("every ability is listed")
    }
}

/// Where the player's abilities stand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Book {
    /// The world tick each can be used again.
    ready_at: [u64; Ability::ALL.len()],
    /// The world tick the guard drops.
    guard_until: u64,
    /// Asked for, and not yet used.
    queued: Vec<Ability>,
    /// A flurry under way: at whom, and how many blows are left.
    flurry: Option<(usize, u8)>,
}

impl Book {
    /// Forget what was asked for and what was under way; the times
    /// until each is ready stay.
    pub(crate) fn drop_all(&mut self) {
        self.queued.clear();
        self.flurry = None;
        self.guard_until = 0;
    }

    /// Whether something here wants a tick of the player's.
    pub(crate) fn pending(&self) -> bool {
        !self.queued.is_empty() || self.flurry.is_some()
    }

    pub(crate) fn cooling(&self, now: u64) -> bool {
        self.guard_until > now || self.ready_at.iter().any(|&at| at > now)
    }
}

impl Sim {
    /// World ticks until an ability can be used again: 0 when it can.
    pub fn cooldown_left(&self, ability: Ability) -> u64 {
        self.book.ready_at[ability.index()].saturating_sub(self.ticks)
    }

    /// World ticks of guard left: 0 with none up.
    pub fn guard_left(&self) -> u64 {
        self.book.guard_until.saturating_sub(self.ticks)
    }

    /// Whether any ability's time is still running down, or a guard is up.
    pub fn cooling(&self) -> bool {
        self.book.cooling(self.ticks)
    }

    /// Whether an ability has been asked for and is waiting to be used.
    pub fn queued(&self, ability: Ability) -> bool {
        self.book.queued.contains(&ability)
    }

    /// Whether the player has anything at all to do on their next tick:
    /// busy, finishing a swing, or with an ability waiting.
    pub fn acting(&self) -> bool {
        self.player.busy() || self.player.swung || self.book.pending()
    }

    /// Ask for an ability. It is used on the player's next tick (one
    /// aimed at a monster, once they are beside it). Asked for with no
    /// monster to aim at, it goes for the nearest one that is after
    /// them; with none of those, nothing happens but the saying so.
    pub fn use_ability(&mut self, ability: Ability) {
        if self.cooldown_left(ability) > 0 || self.queued(ability) {
            return;
        }
        if ability.aimed() && self.target().is_none() {
            let at = self.player.tile;
            let foes = self.monsters.iter().enumerate().filter(|(_, m)| m.alive() && m.angry);
            let Some((nearest, _)) = foes.min_by_key(|(_, m)| (m.tile.0 - at.0).abs().max((m.tile.1 - at.1).abs())) else {
                self.events.push(Event::NoTarget);
                return;
            };
            self.attack(nearest);
        }
        self.book.queued.push(ability);
    }

    /// The player's tick for their abilities: the next blow of a flurry
    /// under way, then everything asked for that can be used now.
    pub(crate) fn use_abilities(&mut self) {
        if let Some((target, left)) = self.book.flurry {
            let there = self.target() == Some(target) && in_reach(self.player.tile, self.monsters[target].tile);
            self.book.flurry = (there && left > 1).then_some((target, left - 1));
            if there {
                self.flurry_blow(target);
            }
        }
        for ability in std::mem::take(&mut self.book.queued) {
            if !ability.aimed() {
                self.used(ability);
                self.book.guard_until = self.ticks + GUARD_TICKS;
                self.events.push(Event::Guarding);
                continue;
            }
            let Some(target) = self.target() else {
                // Whoever it was for is dead, or let go.
                continue;
            };
            if !in_reach(self.player.tile, self.monsters[target].tile) || self.player.moved() {
                // Not there yet: it waits.
                self.book.queued.push(ability);
                continue;
            }
            self.used(ability);
            self.player.swung = true;
            self.last_fight = Some(self.ticks);
            self.monsters[target].angry = true;
            match ability {
                Ability::HeavyStrike => {
                    // From the most a plain blow does to twice that.
                    let most = self.max_hit();
                    let damage = most + (self.rng.next_u64() % (most as u64 + 1)) as u32;
                    self.wound(target, damage);
                }
                Ability::Flurry => {
                    self.book.flurry = Some((target, FLURRY_BLOWS - 1));
                    self.flurry_blow(target);
                }
                Ability::Kick => {
                    self.monsters[target].stunned_until = self.ticks + STUN_TICKS;
                    self.events.push(Event::Stunned { monster: target });
                    self.gain(Skill::Melee, MELEE_XP);
                }
                Ability::Guard => {}
            }
        }
    }

    fn used(&mut self, ability: Ability) {
        self.book.ready_at[ability.index()] = self.ticks + ability.cooldown();
    }

    /// One of a flurry's blows: as likely to land as a plain one, for
    /// half as much at most.
    fn flurry_blow(&mut self, target: usize) {
        self.player.swung = true;
        self.last_fight = Some(self.ticks);
        let defence = self.monsters[target].kind.stats().defence;
        if !self.rng.chance(hit_chance(self.skills.level(Skill::Melee), defence)) {
            self.events.push(Event::Miss { on: Who::Monster(target) });
            return;
        }
        let most = (self.max_hit() / 2).max(1);
        let damage = 1 + (self.rng.next_u64() % most as u64) as u32;
        self.wound(target, damage);
    }

    /// Whether an ability's blow falls on the player's next tick,
    /// should nobody move.
    pub(crate) fn ability_due(&self) -> bool {
        let there = |target: usize| self.target() == Some(target) && in_reach(self.player.tile, self.monsters[target].tile);
        self.book.flurry.is_some_and(|(target, _)| there(target)) || (self.book.queued.iter().any(|a| a.aimed()) && self.target().is_some_and(there))
    }
}

#[cfg(test)]
#[path = "abilities_tests.rs"]
mod tests;
