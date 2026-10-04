//! Changing the world by hand: what the map editor's tools do, and the
//! history that takes them back. Nothing here knows about a pointer or
//! a window, so every tool is tested as plain data.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::terrain::Terrain;
use crate::world::{PropKind, Tile, World};

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
            Some(_) => Tile { terrain, prop: world.prop_at(x, y).map(|p| p.kind) },
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
        history.set(world, x, y, Tile { terrain: Some(terrain), prop: world.prop_at(x, y).map(|p| p.kind) });
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
        history.set(world, x, y, Tile { terrain, prop: Some(kind) });
    }
    terrain.is_some()
}

/// Take away what stands on a tile. `false` when nothing did.
pub fn remove(history: &mut History, world: &mut World, x: i32, y: i32) -> bool {
    let standing = world.prop_at(x, y).is_some();
    if standing {
        history.set(world, x, y, Tile { terrain: world.terrain(x, y), prop: None });
    }
    standing
}

#[cfg(test)]
mod tests {
    use super::*;
    use Terrain::{Dirt, Grass, Sand, Water};

    /// A 10 by 10 field of grass.
    fn field() -> World {
        let mut world = World::new();
        for y in 0..10 {
            for x in 0..10 {
                world.set_terrain(x, y, Grass);
            }
        }
        world
    }

    fn snapshot(world: &World) -> Vec<Tile> {
        (-2..12).flat_map(|y| (-2..12).map(move |x| (x, y))).map(|(x, y)| world.tile(x, y)).collect()
    }

    fn count(world: &World, terrain: Terrain) -> usize {
        snapshot(world).iter().filter(|t| t.terrain == Some(terrain)).count()
    }

    #[test]
    fn brushes_are_round_symmetrical_and_the_size_they_say() {
        assert_eq!(brush((4.7, 2.1), 1), vec![(4, 2)]);
        // Two across sits on the nearest corner: the four tiles around it.
        let mut two = brush((4.7, 2.1), 2);
        two.sort();
        assert_eq!(two, vec![(4, 1), (4, 2), (5, 1), (5, 2)]);
        for size in 1..=9 {
            let tiles = brush((0.5, 0.5), size);
            let (xs, ys): (Vec<i32>, Vec<i32>) = tiles.iter().copied().unzip();
            let across = |v: &[i32]| v.iter().max().unwrap() - v.iter().min().unwrap() + 1;
            assert_eq!((across(&xs), across(&ys)), (size as i32, size as i32), "size {size}");
            // Mirrored left to right and top to bottom about its middle.
            let (sx, sy) = (xs.iter().min().unwrap() + xs.iter().max().unwrap(), ys.iter().min().unwrap() + ys.iter().max().unwrap());
            assert!(tiles.iter().all(|&(x, y)| tiles.contains(&(sx - x, y)) && tiles.contains(&(x, sy - y))), "size {size}");
            // Round: once it is big enough to have corners, they are cut.
            let square = (size * size) as usize;
            assert!(if size >= 4 { tiles.len() < square } else { tiles.len() == square }, "size {size}: {} tiles", tiles.len());
        }
    }

    #[test]
    fn a_stroke_is_one_step_and_undo_puts_everything_back() {
        let mut world = field();
        let before = snapshot(&world);
        let mut history = History::new();
        history.begin();
        paint_line(&mut history, &mut world, (1.5, 1.5), (8.5, 6.5), 3, Some(Dirt));
        assert!(history.commit());
        let after = snapshot(&world);
        assert!(count(&world, Dirt) > 20);

        assert!(history.undo(&mut world));
        assert_eq!(snapshot(&world), before);
        assert!(!history.undo(&mut world), "it was one step");
        assert!(history.redo(&mut world));
        assert_eq!(snapshot(&world), after);
        assert!(!history.redo(&mut world));
    }

    #[test]
    fn a_fast_drag_leaves_no_gaps() {
        let mut world = field();
        let mut history = History::new();
        paint_line(&mut history, &mut world, (0.5, 0.5), (9.5, 9.5), 1, Some(Dirt));
        // Every row and every column the drag crossed has dirt in it.
        for i in 0..10 {
            assert!((0..10).any(|x| world.terrain(x, i) == Some(Dirt)), "row {i}");
            assert!((0..10).any(|y| world.terrain(i, y) == Some(Dirt)), "column {i}");
        }
    }

    #[test]
    fn painting_keeps_what_stands_and_erasing_takes_it() {
        let mut world = field();
        world.place_prop(PropKind::Tree, 5, 5);
        let mut history = History::new();
        paint(&mut history, &mut world, (5.5, 5.5), 3, Some(Sand));
        assert_eq!(world.tile(5, 5), Tile { terrain: Some(Sand), prop: Some(PropKind::Tree) });
        paint(&mut history, &mut world, (5.5, 5.5), 1, None);
        assert_eq!(world.tile(5, 5), Tile::default());
        // Undo brings the tree back with its ground.
        history.undo(&mut world);
        assert_eq!(world.tile(5, 5), Tile { terrain: Some(Grass), prop: Some(PropKind::Tree) });
    }

    #[test]
    fn painting_past_the_edge_makes_new_ground() {
        let mut world = field();
        let mut history = History::new();
        paint(&mut history, &mut world, (-0.5, -0.5), 1, Some(Water));
        assert_eq!(world.terrain(-1, -1), Some(Water));
        history.undo(&mut world);
        assert_eq!(world.terrain(-1, -1), None);
    }

    #[test]
    fn a_step_that_changes_nothing_is_not_a_step() {
        let mut world = field();
        let mut history = History::new();
        history.begin();
        paint(&mut history, &mut world, (3.5, 3.5), 3, Some(Grass));
        assert!(!history.commit());
        // Changed and changed back within one step: nothing either.
        history.begin();
        paint(&mut history, &mut world, (3.5, 3.5), 1, Some(Dirt));
        paint(&mut history, &mut world, (3.5, 3.5), 1, Some(Grass));
        assert!(!history.commit());
        assert!(!history.can_undo());
    }

    #[test]
    fn a_new_edit_forgets_what_was_undone() {
        let mut world = field();
        let mut history = History::new();
        for terrain in [Dirt, Sand] {
            history.begin();
            paint(&mut history, &mut world, (2.5, 2.5), 1, Some(terrain));
            history.commit();
        }
        history.undo(&mut world);
        assert!(history.can_redo());
        history.begin();
        paint(&mut history, &mut world, (7.5, 7.5), 1, Some(Water));
        history.commit();
        assert!(!history.can_redo());
        assert_eq!(world.terrain(2, 2), Some(Dirt));
    }

    #[test]
    fn the_history_forgets_its_oldest_steps_past_the_limit() {
        let mut world = field();
        let mut history = History::new();
        for i in 0..HISTORY_LIMIT + 10 {
            history.begin();
            paint(&mut history, &mut world, (0.5, 0.5), 1, Some(if i % 2 == 0 { Dirt } else { Sand }));
            history.commit();
        }
        let mut steps = 0;
        while history.undo(&mut world) {
            steps += 1;
        }
        assert_eq!(steps, HISTORY_LIMIT);
    }

    #[test]
    fn fill_floods_one_joined_stretch_and_stops_at_other_ground() {
        let mut world = field();
        // A wall of dirt down column 4 splits the field in two.
        for y in 0..10 {
            world.set_terrain(4, y, Dirt);
        }
        world.place_prop(PropKind::Rock, 1, 1);
        let mut history = History::new();
        history.begin();
        assert_eq!(fill(&mut history, &mut world, 0, 0, Water), 40);
        history.commit();
        assert_eq!(count(&world, Water), 40);
        assert_eq!(count(&world, Grass), 50, "the other side of the wall is untouched");
        assert_eq!(world.prop_at(1, 1).map(|p| p.kind), Some(PropKind::Rock), "what stood there still stands");
        // Tiles touching only at a corner are not joined.
        world.set_terrain(4, 0, Grass);
        world.set_terrain(5, 0, Dirt);
        assert_eq!(fill(&mut history, &mut world, 4, 0, Sand), 1);
        // No ground, or the same ground: nothing to do.
        assert_eq!(fill(&mut history, &mut world, -5, -5, Sand), 0);
        assert_eq!(fill(&mut history, &mut world, 0, 0, Water), 0);
        // And the whole flood is one step.
        history.commit();
        history.undo(&mut world);
        history.undo(&mut world);
        assert_eq!(count(&world, Water), 0);
    }

    #[test]
    fn placing_replaces_and_removing_removes_each_as_a_step() {
        let mut world = field();
        let mut history = History::new();
        history.begin();
        assert!(place(&mut history, &mut world, 3, 3, PropKind::Tree));
        assert!(place(&mut history, &mut world, 3, 3, PropKind::Rock));
        assert!(!place(&mut history, &mut world, -9, -9, PropKind::Tree), "no ground there");
        history.commit();
        assert_eq!(world.prop_at(3, 3).map(|p| p.kind), Some(PropKind::Rock));
        history.begin();
        assert!(remove(&mut history, &mut world, 3, 3));
        assert!(!remove(&mut history, &mut world, 3, 3));
        history.commit();
        assert_eq!(world.prop_at(3, 3), None);
        history.undo(&mut world);
        assert_eq!(world.prop_at(3, 3).map(|p| p.kind), Some(PropKind::Rock));
        history.undo(&mut world);
        assert_eq!(world.prop_at(3, 3), None);
    }

    #[test]
    fn undo_in_the_middle_of_a_stroke_takes_the_stroke_so_far() {
        let mut world = field();
        let before = snapshot(&world);
        let mut history = History::new();
        history.begin();
        paint(&mut history, &mut world, (2.5, 2.5), 3, Some(Dirt));
        assert!(history.undo(&mut world));
        assert_eq!(snapshot(&world), before);
    }
}
