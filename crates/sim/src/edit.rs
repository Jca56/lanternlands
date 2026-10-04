//! Changing the world by hand: what the map editor's tools do, and the
//! history that takes them back. Nothing here knows about a pointer or
//! a window, so every tool is tested as plain data.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::terrain::Terrain;
use crate::world::{Marker, PropKind, Tile, World};

/// How many steps back the history goes.
pub const HISTORY_LIMIT: usize = 500;

/// One tile as it was and as it became.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Change {
    x: i32,
    y: i32,
    before: Tile,
    after: Tile,
}

/// Edits as steps that can be taken back and put again. Every change
/// to the world goes through [`History::set`] between a [`History::begin`]
/// and a [`History::commit`]: a whole brush stroke is one step.
#[derive(Default)]
pub struct History {
    undone: Vec<Vec<Change>>,
    done: VecDeque<Vec<Change>>,
    open: Option<Step>,
}

/// A step being made.
#[derive(Default)]
struct Step {
    changes: Vec<Change>,
    /// Where each changed tile's entry is in `changes`.
    places: HashMap<(i32, i32), usize>,
}

impl History {
    pub fn new() -> History {
        History::default()
    }

    /// Start a step. One already open is committed first.
    pub fn begin(&mut self) {
        self.commit();
        self.open = Some(Step::default());
    }

    /// Make a tile `after`, as part of the open step (one is begun if
    /// none is). A tile changed twice in a step remembers how it was
    /// before the first.
    pub fn set(&mut self, world: &mut World, x: i32, y: i32, after: Tile) {
        let before = world.tile(x, y);
        // Without ground nothing stands: say so, so the step compares right.
        let after = if after.terrain.is_none() { Tile::default() } else { after };
        if before == after {
            return;
        }
        world.set_tile(x, y, after);
        let step = self.open.get_or_insert_with(Step::default);
        match step.places.get(&(x, y)) {
            Some(&at) => step.changes[at].after = after,
            None => {
                step.places.insert((x, y), step.changes.len());
                step.changes.push(Change { x, y, before, after });
            }
        }
    }

    /// End the open step. One that changed nothing in the end leaves no
    /// trace; any other makes what was undone unreachable. Returns
    /// whether a step was added.
    pub fn commit(&mut self) -> bool {
        let Some(Step { mut changes, .. }) = self.open.take() else {
            return false;
        };
        changes.retain(|c| c.before != c.after);
        if changes.is_empty() {
            return false;
        }
        self.undone.clear();
        self.done.push_back(changes);
        if self.done.len() > HISTORY_LIMIT {
            self.done.pop_front();
        }
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Take the last step back. The open step is committed first.
    pub fn undo(&mut self, world: &mut World) -> bool {
        self.commit();
        let Some(changes) = self.done.pop_back() else {
            return false;
        };
        for c in changes.iter().rev() {
            world.set_tile(c.x, c.y, c.before);
        }
        self.undone.push(changes);
        true
    }

    /// Put back the step last undone.
    pub fn redo(&mut self, world: &mut World) -> bool {
        if self.open.is_some() {
            return false;
        }
        let Some(changes) = self.undone.pop() else {
            return false;
        };
        for c in &changes {
            world.set_tile(c.x, c.y, c.after);
        }
        self.done.push_back(changes);
        true
    }
}

/// The tiles a round brush `size` tiles across covers with its middle at
/// `center` (in tiles). An odd brush sits on the tile under the centre;
/// an even one on the corner nearest it, so both stay symmetrical.
pub fn brush(center: (f64, f64), size: u32) -> Vec<(i32, i32)> {
    let size = size.max(1);
    let radius = size as f64 / 2.0;
    let snap = |v: f64| if size % 2 == 1 { v.floor() + 0.5 } else { v.round() };
    let (cx, cy) = (snap(center.0), snap(center.1));
    let reach = radius.ceil() as i32 + 1;
    let (tx, ty) = (cx.floor() as i32, cy.floor() as i32);
    let mut tiles = Vec::new();
    for y in ty - reach..=ty + reach {
        for x in tx - reach..=tx + reach {
            let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
            if dx * dx + dy * dy <= radius * radius {
                tiles.push((x, y));
            }
        }
    }
    tiles
}

/// Paint ground with the brush at `center`: `Some` lays that ground
/// (and leaves what stands on it), `None` takes the ground away, and
/// with it whatever stood there.
pub fn paint(history: &mut History, world: &mut World, center: (f64, f64), size: u32, terrain: Option<Terrain>) {
    for (x, y) in brush(center, size) {
        let tile = match terrain {
            Some(_) => Tile { terrain, ..world.tile(x, y) },
            None => Tile::default(),
        };
        history.set(world, x, y, tile);
    }
}

/// Paint along a straight drag from `from` to `to`, close enough
/// together that a fast pointer leaves no gaps.
pub fn paint_line(history: &mut History, world: &mut World, from: (f64, f64), to: (f64, f64), size: u32, terrain: Option<Terrain>) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = (dx.hypot(dy) * 2.0).ceil().max(1.0) as u32;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        paint(history, world, (from.0 + dx * t, from.1 + dy * t), size, terrain);
    }
}

/// Flood the stretch of one kind of ground that tile `(x, y)` is part
/// of (tiles joined by their sides) with `terrain`. Returns how many
/// tiles changed; none where there is no ground, or it is that ground
/// already.
pub fn fill(history: &mut History, world: &mut World, x: i32, y: i32, terrain: Terrain) -> usize {
    let Some(from) = world.terrain(x, y).filter(|t| *t != terrain) else {
        return 0;
    };
    let mut seen = HashSet::from([(x, y)]);
    let mut queue = VecDeque::from([(x, y)]);
    while let Some((x, y)) = queue.pop_front() {
        history.set(world, x, y, Tile { terrain: Some(terrain), ..world.tile(x, y) });
        for next in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if world.terrain(next.0, next.1) == Some(from) && seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    seen.len()
}

/// Stand a prop on a tile, in place of whatever stood there. `false`
/// where there is no ground.
pub fn place(history: &mut History, world: &mut World, x: i32, y: i32, kind: PropKind) -> bool {
    let terrain = world.terrain(x, y);
    if terrain.is_some() {
        history.set(world, x, y, Tile { prop: Some(kind), ..world.tile(x, y) });
    }
    terrain.is_some()
}

/// Take away what stands on a tile. `false` when nothing did.
pub fn remove(history: &mut History, world: &mut World, x: i32, y: i32) -> bool {
    let standing = world.prop_at(x, y).is_some();
    if standing {
        history.set(world, x, y, Tile { prop: None, ..world.tile(x, y) });
    }
    standing
}

/// Move what stands on tile `from` to tile `to`. `false`, and nothing
/// changes, when nothing stands on `from`, or `to` has no ground or
/// something standing on it already (or is `from` itself).
pub fn move_prop(history: &mut History, world: &mut World, from: (i32, i32), to: (i32, i32)) -> bool {
    let Some(prop) = world.prop_at(from.0, from.1) else {
        return false;
    };
    if world.terrain(to.0, to.1).is_none() || world.prop_at(to.0, to.1).is_some() {
        return false;
    }
    history.set(world, from.0, from.1, Tile { prop: None, ..world.tile(from.0, from.1) });
    history.set(world, to.0, to.1, Tile { prop: Some(prop.kind), ..world.tile(to.0, to.1) });
    true
}

/// Put a marker on a tile, taking it from wherever it was. `false`
/// where there is no ground.
pub fn set_marker(history: &mut History, world: &mut World, marker: Marker, x: i32, y: i32) -> bool {
    if world.terrain(x, y).is_none() {
        return false;
    }
    if let Some((ox, oy)) = world.marker(marker).filter(|&at| at != (x, y)) {
        history.set(world, ox, oy, Tile { marker: None, ..world.tile(ox, oy) });
    }
    history.set(world, x, y, Tile { marker: Some(marker), ..world.tile(x, y) });
    true
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
