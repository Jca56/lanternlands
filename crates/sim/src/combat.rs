//! Fighting: the player's blows and the monsters', health and how it
//! comes back, and what dying costs.
//!
//! The player and the world keep time separately. The world's tick
//! moves the monsters and heals; the player's moves the player and
//! lands their blows. They run at the same pace, so a fight is fair,
//! but the player's can start the moment they act instead of waiting
//! for the world's to come round.

use crate::items::Inventory;
use crate::monsters::Monster;
use crate::path;
use crate::player::{Activity, Event, Intent, Sim, Who, start_tile};
use crate::skills::Skill;

/// Ticks from one of the player's bare-handed blows to the next.
pub const ATTACK_TICKS: u32 = 3;
/// World ticks after the last blow given or taken that still count as
/// being in a fight.
pub const COMBAT_TICKS: u64 = 12;
/// In a fight, health comes back by one every this many world ticks.
const TRICKLE_TICKS: u64 = 8;
/// Out of one, by a twentieth of the whole every tick.
const RECOVERY_SHARE: u32 = 20;
/// Experience for each point of damage dealt, and for each blow avoided.
const MELEE_XP: u32 = 4;
const VITALITY_XP: u32 = 1;
const DEFENCE_XP: u32 = 4;

/// What was left where the player died.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grave {
    pub tile: (i32, i32),
    pub items: Inventory,
}

/// The chance a blow lands: better the more the attacker's level is
/// over the defender's, never certain and never hopeless.
pub fn hit_chance(attack: u8, defence: u8) -> f64 {
    (0.60 + 0.04 * (attack as f64 - defence as f64)).clamp(0.10, 0.95)
}

/// Close enough to strike: on the same tile or one of the eight around.
fn in_reach(a: (i32, i32), b: (i32, i32)) -> bool {
    (a.0 - b.0).abs() <= 1 && (a.1 - b.1).abs() <= 1
}

/// The eight ways a step can go.
const STEPS: [(i32, i32); 8] = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)];

fn apart(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

impl Sim {
    /// The most health the player can have.
    pub fn max_health(&self) -> u32 {
        20 + 6 * (self.skills.level(Skill::Vitality) as u32 - 1)
    }

    /// The most one of the player's blows does.
    pub fn max_hit(&self) -> u32 {
        2 + self.skills.level(Skill::Melee) as u32 / 3
    }

    /// Whether a blow was given or taken lately.
    pub fn in_combat(&self) -> bool {
        self.last_fight.is_some_and(|at| self.ticks < at + COMBAT_TICKS)
    }

    /// Set the player on a monster: to it, and then at it.
    pub fn attack(&mut self, monster: usize) {
        if self.monsters.get(monster).is_some_and(Monster::alive) {
            self.stop();
            self.player.intent = Some(Intent::Attack(monster));
            self.engage(false);
        }
    }

    /// The monster the player is set on: going for it, or fighting it.
    pub fn target(&self) -> Option<usize> {
        let target = match (self.player.intent, self.player.activity) {
            (_, Some(Activity::Fighting { target, .. })) | (Some(Intent::Attack(target)), _) => target,
            _ => return None,
        };
        self.monsters.get(target).is_some_and(Monster::alive).then_some(target)
    }

    /// Whether the player's next tick is a swing: a blow at what they
    /// fight, or the end of a swing at what they work.
    pub fn swing_due(&self) -> bool {
        match self.player.activity {
            Some(Activity::Working { at, swing }) => self.node_at(at).is_some_and(|node| swing + 1 >= node.swing_ticks),
            Some(Activity::Fighting { target, cooldown }) => cooldown == 0 && self.monsters.get(target).is_some_and(|m| m.alive() && in_reach(self.player.tile, m.tile)),
            None => false,
        }
    }

    /// Whether a monster's next tick is a blow at the player, should
    /// they stay where they are.
    pub fn monster_swing_due(&self, monster: usize) -> bool {
        self.monsters.get(monster).is_some_and(|m| m.alive() && m.angry && m.cooldown <= 1 && m.tile != self.player.tile && in_reach(m.tile, self.player.tile))
    }

    /// Whether somebody other than monster `but` stands on `tile`: the
    /// player, or another living monster.
    fn taken(&self, tile: (i32, i32), but: usize) -> bool {
        tile == self.player.tile || self.monsters.iter().enumerate().any(|(i, m)| i != but && m.alive() && m.tile == tile)
    }

    /// Gain experience, and say so.
    pub(crate) fn gain(&mut self, skill: Skill, amount: u32) {
        self.events.push(Event::Xp { skill, amount });
        if let Some(level) = self.skills.gain(skill, amount) {
            self.events.push(Event::LevelUp { skill, level });
        }
    }

    /// On the way to a monster: strike once in reach, and until then
    /// keep heading for where it is now. `stepping` on the player's own
    /// tick, when they may just have taken the last step of the way.
    pub(crate) fn engage(&mut self, stepping: bool) {
        let Some(Intent::Attack(target)) = self.player.intent else {
            return;
        };
        let Some(at) = self.monsters.get(target).filter(|m| m.alive()).map(|m| m.tile) else {
            return self.stop();
        };
        if in_reach(self.player.tile, at) {
            // The first blow lands when they have got there: a step
            // taken this tick is only done by the next.
            let cooldown = if stepping && self.player.moved() { 1 } else { 0 };
            self.stop();
            self.player.activity = Some(Activity::Fighting { target, cooldown });
            return;
        }
        // Beside it is near enough.
        let way = self.way_to(at);
        if way.is_empty() {
            // No nearer to be got.
            return self.stop();
        }
        self.player.goal = way.last().copied();
        self.player.path = way.into();
    }

    /// A player's tick of fighting: a blow when one is due.
    pub(crate) fn fight(&mut self) {
        let Some(Activity::Fighting { target, cooldown }) = self.player.activity else {
            return;
        };
        let Some(monster) = self.monsters.get(target).filter(|m| m.alive()) else {
            self.player.activity = None;
            return;
        };
        if !in_reach(self.player.tile, monster.tile) {
            // It moved off: after it.
            self.player.activity = None;
            self.player.intent = Some(Intent::Attack(target));
            return self.engage(false);
        }
        if cooldown > 0 {
            self.player.activity = Some(Activity::Fighting { target, cooldown: cooldown - 1 });
            return;
        }
        self.player.activity = Some(Activity::Fighting { target, cooldown: ATTACK_TICKS - 1 });
        self.player.swung = true;
        self.last_fight = Some(self.ticks);
        let stats = monster.kind.stats();
        let landed = self.rng.chance(hit_chance(self.skills.level(Skill::Melee), stats.defence));
        // Struck at, it knows who its enemy is, however far it could see.
        self.monsters[target].angry = true;
        if !landed {
            self.events.push(Event::Miss { on: Who::Monster(target) });
            return;
        }
        let rolled = 1 + (self.rng.next_u64() % self.max_hit() as u64) as u32;
        let damage = rolled.min(self.monsters[target].health);
        self.monsters[target].health -= damage;
        self.events.push(Event::Hit { on: Who::Monster(target), damage });
        self.gain(Skill::Melee, MELEE_XP * damage);
        self.gain(Skill::Vitality, VITALITY_XP * damage);
        if self.monsters[target].health == 0 {
            self.slay(target);
        }
    }

    /// A monster falls: it is gone until its time is up, and what it
    /// leaves goes into the bag.
    fn slay(&mut self, target: usize) {
        let monster = &mut self.monsters[target];
        let stats = monster.kind.stats();
        monster.back_at = Some(self.ticks + stats.respawn_ticks);
        monster.angry = false;
        self.player.activity = None;
        self.events.push(Event::Slain { monster: target });
        if let Some((item, chance)) = stats.drop
            && self.rng.chance(chance)
        {
            if self.inventory.add(item, 1) == 0 {
                self.events.push(Event::Got { item, count: 1 });
            } else {
                self.events.push(Event::InventoryFull);
            }
        }
    }

    /// The world's tick for every monster: the dead come back, the
    /// angry chase and strike, the calm go home and wander. A tile
    /// holds one body: none steps onto the player or another monster.
    pub(crate) fn monsters_act(&mut self) {
        for i in 0..self.monsters.len() {
            let (now, player) = (self.ticks, self.player.tile);
            let monster = &mut self.monsters[i];
            let (kind, home, tile) = (monster.kind, monster.home, monster.tile);
            let stats = kind.stats();
            monster.from = tile;
            monster.cooldown = monster.cooldown.saturating_sub(1);
            if let Some(back) = monster.back_at {
                // Back at home, once its time is up and nobody is
                // standing there.
                if now >= back && !self.taken(home, i) {
                    self.monsters[i] = Monster::new(kind, home);
                }
                continue;
            }
            if tile == player {
                // However that came about (the player woke here, or
                // the map changed), it steps off them.
                self.make_way(i);
                continue;
            }
            // It goes for a player it can see, and gives up on one who
            // has drawn it too far from home.
            if apart(player, home) > stats.leash {
                monster.angry = false;
            } else if apart(player, tile) <= stats.sight {
                monster.angry = true;
            }
            if !monster.angry {
                // Left alone it mends, and wanders about its home.
                monster.health = stats.health;
                self.idle(i);
                continue;
            }
            if !in_reach(tile, player) {
                let next = path::find_around(&self.world, tile, player, &|at| self.taken(at, i)).first().copied();
                if let Some(next) = next.filter(|&next| !self.taken(next, i)) {
                    self.monsters[i].tile = next;
                }
                continue;
            }
            if monster.cooldown > 0 {
                continue;
            }
            monster.cooldown = stats.attack_ticks;
            monster.swung_at = Some(now);
            self.last_fight = Some(now);
            // Set upon while doing nothing, the player fights back.
            if !self.player.busy() {
                self.player.activity = Some(Activity::Fighting { target: i, cooldown: 0 });
            }
            if !self.rng.chance(hit_chance(stats.attack, self.skills.level(Skill::Defence))) {
                self.events.push(Event::Miss { on: Who::Player });
                self.gain(Skill::Defence, DEFENCE_XP);
                continue;
            }
            let rolled = 1 + (self.rng.next_u64() % stats.max_hit as u64) as u32;
            let damage = rolled.min(self.player.health);
            self.player.health -= damage;
            self.events.push(Event::Hit { on: Who::Player, damage });
            if self.player.health == 0 {
                return self.die();
            }
        }
    }

    /// A monster steps off the tile it shares onto any free one beside.
    fn make_way(&mut self, i: usize) {
        let tile = self.monsters[i].tile;
        let free = STEPS.iter().find(|&&step| path::can_step(&self.world, tile, step) && !self.taken((tile.0 + step.0, tile.1 + step.1), i));
        if let Some(step) = free {
            self.monsters[i].tile = (tile.0 + step.0, tile.1 + step.1);
        }
    }

    /// A calm monster's tick: back toward home if it is far from it,
    /// else now and then a step somewhere near.
    fn idle(&mut self, i: usize) {
        let monster = &self.monsters[i];
        let stats = monster.kind.stats();
        let (tile, home) = (monster.tile, monster.home);
        if apart(tile, home) > stats.roam {
            let next = path::find_around(&self.world, tile, home, &|at| self.taken(at, i)).first().copied();
            if let Some(next) = next.filter(|&next| !self.taken(next, i)) {
                self.monsters[i].tile = next;
            }
            return;
        }
        if self.ticks < monster.wander_at {
            return;
        }
        // Every few ticks, a step one of the eight ways, if it can be
        // taken and keeps it near home.
        self.monsters[i].wander_at = self.ticks + 5 + self.rng.next_u64() % 8;
        let step = STEPS[(self.rng.next_u64() % 8) as usize];
        let next = (tile.0 + step.0, tile.1 + step.1);
        if path::can_step(&self.world, tile, step) && apart(next, home) <= stats.roam && !self.taken(next, i) {
            self.monsters[i].tile = next;
        }
    }

    /// The world's tick for the player's health: a trickle in a fight,
    /// quickly back to full out of one.
    pub(crate) fn recover(&mut self) {
        let max = self.max_health();
        if self.player.health >= max {
            self.player.health = max;
        } else if !self.in_combat() {
            self.player.health = (self.player.health + max.div_ceil(RECOVERY_SHARE)).min(max);
        } else if self.ticks.is_multiple_of(TRICKLE_TICKS) {
            self.player.health += 1;
        }
    }

    /// The player dies: what they carried stays in a grave where they
    /// fell, an older grave is lost with all that was in it, and they
    /// wake at the start, whole, with every monster calm again.
    fn die(&mut self) {
        self.events.push(Event::Died);
        if self.grave.take().is_some() {
            self.events.push(Event::GraveLost);
        }
        if !self.inventory.is_empty() {
            self.grave = Some(Grave { tile: self.player.tile, items: std::mem::take(&mut self.inventory) });
        }
        self.stop();
        self.player.tile = start_tile(&self.world);
        self.player.from = self.player.tile;
        self.player.health = self.max_health();
        self.last_fight = None;
        for monster in &mut self.monsters {
            monster.angry = false;
        }
    }

    /// Take back what the grave holds, as much as there is room for.
    /// The grave goes when it is empty.
    pub(crate) fn open_grave(&mut self) {
        let Some(mut grave) = self.grave.take() else {
            return;
        };
        let mut left = Inventory::default();
        for stack in grave.items.slots().iter().flatten() {
            let over = self.inventory.add(stack.item, stack.count);
            left.add(stack.item, over);
        }
        let all = left.is_empty();
        self.events.push(Event::GraveOpened { all });
        if !all {
            grave.items = left;
            self.grave = Some(grave);
        }
    }
}

#[cfg(test)]
#[path = "combat_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "combat_bodies_tests.rs"]
mod bodies;
