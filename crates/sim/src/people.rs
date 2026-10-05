//! The people of the world: who they are and what they have to say,
//! and a person as the simulation keeps them.
//!
//! A person lives on a tile the map gives them. The player walks up to
//! one and they talk: what they say comes a line at a time, and walking
//! off ends it. People are in nobody's fight (nothing strikes them, and
//! they strike nothing), but they take up their tile like anybody else.

use crate::combat::{STEPS, apart, in_reach};
use crate::path;
use crate::player::{Intent, Sim};
use crate::world::World;

/// Ticks from one step of someone who wanders to their next: at least
/// this many, and up to this many more.
const WANDER_TICKS: u64 = 8;
const WANDER_SPREAD: u64 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PersonKind {
    Smith,
    Herbalist,
    Guard,
}

impl PersonKind {
    pub const ALL: [PersonKind; 3] = [PersonKind::Smith, PersonKind::Herbalist, PersonKind::Guard];

    /// What they are called in a map file. Never change one.
    pub const fn name(self) -> &'static str {
        match self {
            PersonKind::Smith => "smith",
            PersonKind::Herbalist => "herbalist",
            PersonKind::Guard => "guard",
        }
    }

    pub fn from_name(name: &str) -> Option<PersonKind> {
        PersonKind::ALL.into_iter().find(|k| k.name() == name)
    }

    /// Their name as the player reads it.
    pub const fn label(self) -> &'static str {
        match self {
            PersonKind::Smith => "Bram",
            PersonKind::Herbalist => "Maren",
            PersonKind::Guard => "Odo",
        }
    }

    /// What they do, as the player reads it.
    pub const fn calling(self) -> &'static str {
        match self {
            PersonKind::Smith => "Smith",
            PersonKind::Herbalist => "Herbalist",
            PersonKind::Guard => "Guard",
        }
    }

    /// What they have to say, a line at a time: their own lives and
    /// each other. Nobody tells the player how the game works.
    pub const fn lines(self) -> &'static [&'static str] {
        match self {
            PersonKind::Smith => &[
                "Thirty years at this anvil and my back sounds like a hinge. Don't get old, that's my advice.",
                "Tin's gone up again. Odo says it's the goblins. Odo says everything is the goblins.",
                "Maren keeps leaving flowers on my furnace. They don't last long, but it's the thought.",
            ],
            PersonKind::Herbalist => &[
                "Mind the sprouts! I have been talking that one into growing for a week.",
                "Bram hammers from dawn. I told him my cauldron needs quiet. He laughed, so now his tea tastes of feet.",
                "Odo asked me for something to keep him awake on his round. I told him to try walking faster.",
            ],
            PersonKind::Guard => &[
                "Two steps that way, two steps back. They call it a patrol. I call it a very small walk.",
                "Nothing ever happens here. I'm not complaining. Well, I am a bit.",
                "Bram made this helmet. It's a fine helmet. It is also a size too small, and he knows it.",
            ],
        }
    }

    /// How far from home they wander when left alone, in tiles: none
    /// for those who stay where they are put.
    pub const fn roam(self) -> i32 {
        match self {
            PersonKind::Smith | PersonKind::Herbalist => 0,
            PersonKind::Guard => 2,
        }
    }
}

/// One person in the world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub kind: PersonKind,
    /// The tile they live on: where they stand, or what they stay near.
    pub home: (i32, i32),
    pub tile: (i32, i32),
    /// The tile they were on before the last tick: the same, when they
    /// did not move.
    pub from: (i32, i32),
    /// The tick they next think of taking a step.
    pub(crate) wander_at: u64,
}

impl Person {
    pub fn new(kind: PersonKind, home: (i32, i32)) -> Person {
        Person { kind, home, tile: home, from: home, wander_at: 0 }
    }

    /// Whether the last tick moved them.
    pub fn moved(&self) -> bool {
        self.tile != self.from
    }
}

/// A talk under way: who with, by their place among the simulation's
/// people, and which of their lines is being said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Talk {
    pub(crate) person: usize,
    pub(crate) line: usize,
}

/// What is being said to the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saying {
    /// Who says it, by their place among the simulation's people.
    pub person: usize,
    pub kind: PersonKind,
    pub text: &'static str,
    /// Which of their lines it is, counted from one, and how many they
    /// have.
    pub line: usize,
    pub of: usize,
}

/// A person on every tile the map says one lives on.
pub(crate) fn people_of(world: &World) -> Vec<Person> {
    world.people().into_iter().map(|(kind, x, y)| Person::new(kind, (x, y))).collect()
}

impl Sim {
    /// Whether a person stands on `tile`.
    pub(crate) fn person_on(&self, tile: (i32, i32)) -> bool {
        self.people.iter().any(|p| p.tile == tile)
    }

    /// Whether anybody but the player stands on `tile`: a living
    /// monster, or a person.
    pub(crate) fn someone_on(&self, tile: (i32, i32)) -> bool {
        self.monster_on(tile) || self.person_on(tile)
    }

    /// Set the player to talk with a person: to them, and then a word.
    pub fn talk_to(&mut self, person: usize) {
        let Some(at) = self.people.get(person).map(|p| p.tile) else {
            return;
        };
        // A tile holds one body: the way there ends beside them.
        self.walk_to(at);
        self.player.intent = Some(Intent::Talk(person));
    }

    /// The person the player is on their way to talk with, or talking
    /// with: they wait where they are.
    pub fn awaited(&self) -> Option<usize> {
        match (self.player.intent, self.player.talk) {
            (Some(Intent::Talk(person)), _) => Some(person),
            (_, Some(talk)) => Some(talk.person),
            _ => None,
        }
    }

    /// Having got to whoever they went to talk with, the talk starts.
    pub(crate) fn greet(&mut self) {
        let Some(Intent::Talk(person)) = self.player.intent else {
            return;
        };
        if !self.player.path.is_empty() {
            return;
        }
        self.player.intent = None;
        // Near enough to be heard, or it was no use going.
        if self.people.get(person).is_some_and(|p| in_reach(self.player.tile, p.tile)) {
            self.player.talk = Some(Talk { person, line: 0 });
        }
    }

    /// What is being said to the player, while someone is talking to
    /// them.
    pub fn saying(&self) -> Option<Saying> {
        let talk = self.player.talk?;
        let kind = self.people.get(talk.person)?.kind;
        let lines = kind.lines();
        Some(Saying { person: talk.person, kind, text: lines.get(talk.line)?, line: talk.line + 1, of: lines.len() })
    }

    /// Hear the next thing they have to say. After the last, the talk
    /// is over.
    pub fn talk_on(&mut self) {
        let next = self.saying().filter(|saying| saying.line < saying.of);
        self.player.talk = next.map(|saying| Talk { person: saying.person, line: saying.line });
    }

    /// Turn from whoever is talking: the talk is over.
    pub fn end_talk(&mut self) {
        self.player.talk = None;
    }

    /// The world's tick for every person: those who wander take a step
    /// now and then, never far from home and never onto anybody.
    pub(crate) fn people_act(&mut self) {
        let awaited = self.awaited();
        for i in 0..self.people.len() {
            let person = &mut self.people[i];
            person.from = person.tile;
            let (roam, home, tile) = (person.kind.roam(), person.home, person.tile);
            if roam == 0 || awaited == Some(i) || self.ticks < person.wander_at {
                continue;
            }
            self.people[i].wander_at = self.ticks + WANDER_TICKS + self.rng.next_u64() % WANDER_SPREAD;
            let step = STEPS[(self.rng.next_u64() % 8) as usize];
            let next = (tile.0 + step.0, tile.1 + step.1);
            if path::can_step(&self.world, tile, step) && apart(next, home) <= roam && next != self.player.tile && !self.someone_on(next) {
                self.people[i].tile = next;
            }
        }
    }
}

#[cfg(test)]
#[path = "people_tests.rs"]
mod tests;
