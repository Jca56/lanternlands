//! The people in the world's area of the window: where each is drawn,
//! which one the pointer is on, and which way they face.

use lntrn_math::Vec2;
use sim::TICK_SECONDS;

use super::Game;
use super::view::{GHOST, on_body};
use crate::anim::{Facing, Pose};
use crate::scene::Placed;
use crate::wear::{dress, person_wear};

/// How tall a person is to the pointer, in tiles from their feet.
const AIM_HEIGHT: f64 = 1.45;

impl Game {
    /// Where each person's feet are drawn, in tiles: sliding from the
    /// tile they left to the one they are on over the world's tick.
    pub(super) fn person_grounds(&self) -> Vec<Vec2> {
        let into = (self.world_phase / TICK_SECONDS).clamp(0.0, 1.0);
        let middle = |(x, y): (i32, i32)| Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
        self.sim.people.iter().map(|p| middle(p.from).lerp(middle(p.tile), into)).collect()
    }

    /// The person drawn under a point of the screen: the one nearest
    /// the front, where two overlap.
    pub(super) fn person_under(&self, pointer: Vec2) -> Option<usize> {
        let at = self.camera.to_world(pointer, self.view, self.scale);
        let grounds = self.person_grounds();
        grounds.iter().enumerate().filter(|(_, feet)| on_body(at, **feet, AIM_HEIGHT)).max_by(|a, b| a.1.y.total_cmp(&b.1.y)).map(|(i, _)| i)
    }

    /// Turn each person the way they last stepped, or toward the player
    /// who is coming to talk with them.
    pub(super) fn face_people(&mut self) {
        self.person_facing.resize(self.sim.people.len(), Facing::default());
        let (player, awaited) = (self.sim.player.tile, self.sim.awaited());
        for (i, (person, facing)) in self.sim.people.iter().zip(&mut self.person_facing).enumerate() {
            let way = if awaited == Some(i) { (player.0 - person.tile.0, player.1 - person.tile.1) } else { (person.tile.0 - person.from.0, person.tile.1 - person.from.1) };
            *facing = Facing::of_step(way.0, way.1).unwrap_or(*facing);
        }
    }

    /// Stand the people in `extras`: each where they are, dressed, at a
    /// walk when they move; and while editing, a ghost of each on the
    /// tile they live on instead.
    pub(super) fn stand_people(&self, extras: &mut Vec<Placed>) {
        if self.editing {
            for (kind, x, y) in self.sim.world.people() {
                if let Some((sprite, flip)) = self.look.person(kind).sprite(Facing::Down, Pose::Idle) {
                    extras.push(Placed { sprite, ground: Vec2::new(x as f64 + 0.5, y as f64 + 0.5), flip, tint: [GHOST; 4] });
                }
            }
            return;
        }
        let into = (self.world_phase / TICK_SECONDS).clamp(0.0, 1.0);
        let grounds = self.person_grounds();
        for (i, person) in self.sim.people.iter().enumerate() {
            let pose = Pose::at(person.moved(), self.sim.ticks, into);
            let facing = self.person_facing.get(i).copied().unwrap_or_default();
            let worn: Vec<_> = person_wear(person.kind).iter().map(|&(slot, tint)| (self.look.gear(slot), tint)).collect();
            dress(extras, grounds[i], facing, pose, self.look.person(person.kind), &worn);
        }
    }
}
