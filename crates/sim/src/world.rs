//! The world: tiles kept in chunks, and the things standing on them.
//!
//! Tile `(x, y)` covers the square from `(x, y)` to `(x + 1, y + 1)`; x
//! grows east and y grows south. A tile that was never set is void:
//! there is no ground there at all.

use std::collections::{BTreeSet, HashMap};

use crate::terrain::Terrain;

/// Tiles along one side of a chunk.
pub const CHUNK: i32 = 32;
pub(crate) const AREA: usize = (CHUNK * CHUNK) as usize;
pub(crate) const VOID: u8 = u8::MAX;

/// What a prop is. Placeholders for now: objects get their real
/// definitions from data when they start to do things.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropKind {
    Tree,
    Rock,
    HerbPurple,
    HerbCyan,
}

impl PropKind {
    pub const ALL: [PropKind; 4] = [PropKind::Tree, PropKind::Rock, PropKind::HerbPurple, PropKind::HerbCyan];

    /// What it is called in a map file. Never change one: saved maps
    /// hold these.
    pub const fn name(self) -> &'static str {
        match self {
            PropKind::Tree => "tree",
            PropKind::Rock => "rock",
            PropKind::HerbPurple => "herb_purple",
            PropKind::HerbCyan => "herb_cyan",
        }
    }

    pub fn from_name(name: &str) -> Option<PropKind> {
        PropKind::ALL.into_iter().find(|k| k.name() == name)
    }
}

/// Something standing on a tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Prop {
    pub kind: PropKind,
    pub x: i32,
    pub y: i32,
}

/// Everything about one tile: its ground, and what stands on it. No
/// ground means nothing stands there either.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tile {
    pub terrain: Option<Terrain>,
    pub prop: Option<PropKind>,
}

pub(crate) struct Chunk {
    /// A terrain's rank per tile, row by row, or [`VOID`].
    pub(crate) tiles: Box<[u8; AREA]>,
    pub(crate) props: Vec<Prop>,
}

impl Chunk {
    pub(crate) fn new() -> Chunk {
        Chunk { tiles: Box::new([VOID; AREA]), props: Vec::new() }
    }

    /// No ground anywhere and nothing standing: not worth keeping.
    pub(crate) fn is_empty(&self) -> bool {
        self.props.is_empty() && self.tiles.iter().all(|&t| t == VOID)
    }
}

/// The chunk a tile is in, and its place in that chunk's rows.
fn locate(x: i32, y: i32) -> ((i32, i32), usize) {
    let key = (x.div_euclid(CHUNK), y.div_euclid(CHUNK));
    let index = (y.rem_euclid(CHUNK) * CHUNK + x.rem_euclid(CHUNK)) as usize;
    (key, index)
}

#[derive(Default)]
pub struct World {
    pub(crate) chunks: HashMap<(i32, i32), Chunk>,
    /// Chunks changed since the world was last loaded or saved.
    pub(crate) dirty: BTreeSet<(i32, i32)>,
}

impl World {
    pub fn new() -> World {
        World::default()
    }

    /// Whether anything changed since the world was last loaded or saved.
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// The ground at a tile; `None` where there is none.
    pub fn terrain(&self, x: i32, y: i32) -> Option<Terrain> {
        let (key, index) = locate(x, y);
        Terrain::from_rank(self.chunks.get(&key)?.tiles[index])
    }

    pub fn set_terrain(&mut self, x: i32, y: i32, terrain: Terrain) {
        let (key, index) = locate(x, y);
        self.chunks.entry(key).or_insert_with(Chunk::new).tiles[index] = terrain.rank();
        self.dirty.insert(key);
    }

    /// Take a tile's ground away, and whatever stood on it.
    pub fn clear_tile(&mut self, x: i32, y: i32) {
        let (key, index) = locate(x, y);
        if let Some(chunk) = self.chunks.get_mut(&key) {
            chunk.tiles[index] = VOID;
            chunk.props.retain(|p| (p.x, p.y) != (x, y));
            self.dirty.insert(key);
        }
    }

    pub fn prop_at(&self, x: i32, y: i32) -> Option<Prop> {
        let (key, _) = locate(x, y);
        self.chunks.get(&key)?.props.iter().copied().find(|p| (p.x, p.y) == (x, y))
    }

    /// Stand a prop on a tile. `false`, and nothing changes, when the
    /// tile has no ground or already holds one.
    pub fn place_prop(&mut self, kind: PropKind, x: i32, y: i32) -> bool {
        if self.terrain(x, y).is_none() || self.prop_at(x, y).is_some() {
            return false;
        }
        let (key, _) = locate(x, y);
        self.chunks.get_mut(&key).expect("the tile has ground").props.push(Prop { kind, x, y });
        self.dirty.insert(key);
        true
    }

    pub fn remove_prop(&mut self, x: i32, y: i32) -> Option<Prop> {
        let (key, _) = locate(x, y);
        let props = &mut self.chunks.get_mut(&key)?.props;
        let at = props.iter().position(|p| (p.x, p.y) == (x, y))?;
        self.dirty.insert(key);
        Some(props.swap_remove(at))
    }

    /// Everything about a tile at once.
    pub fn tile(&self, x: i32, y: i32) -> Tile {
        Tile { terrain: self.terrain(x, y), prop: self.prop_at(x, y).map(|p| p.kind) }
    }

    /// Make a tile exactly `tile`. With no ground, nothing stands on it,
    /// whatever `tile.prop` says.
    pub fn set_tile(&mut self, x: i32, y: i32, tile: Tile) {
        let Some(terrain) = tile.terrain else {
            return self.clear_tile(x, y);
        };
        self.set_terrain(x, y, terrain);
        if self.prop_at(x, y).map(|p| p.kind) != tile.prop {
            self.remove_prop(x, y);
            if let Some(kind) = tile.prop {
                self.place_prop(kind, x, y);
            }
        }
    }

    /// Every prop on the tiles from `(x0, y0)` up to, not including,
    /// `(x1, y1)`, in no particular order.
    pub fn props_in(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> impl Iterator<Item = Prop> + '_ {
        let chunks_x = x0.div_euclid(CHUNK)..=(x1 - 1).div_euclid(CHUNK);
        let chunks_y = y0.div_euclid(CHUNK)..=(y1 - 1).div_euclid(CHUNK);
        chunks_y
            .flat_map(move |cy| chunks_x.clone().map(move |cx| (cx, cy)))
            .filter_map(|key| self.chunks.get(&key))
            .flat_map(|chunk| chunk.props.iter().copied())
            .filter(move |p| p.x >= x0 && p.x < x1 && p.y >= y0 && p.y < y1)
    }

    /// The tiles the world's chunks span, as `[x0, y0, x1, y1]` with the
    /// far edges not included; `None` for an empty world.
    pub fn bounds(&self) -> Option<[i32; 4]> {
        let mut keys = self.chunks.keys();
        let &(x, y) = keys.next()?;
        let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
        for &(x, y) in keys {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
        Some([x0 * CHUNK, y0 * CHUNK, (x1 + 1) * CHUNK, (y1 + 1) * CHUNK])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_is_void_until_it_is_set() {
        let mut world = World::new();
        assert_eq!(world.terrain(3, 4), None);
        world.set_terrain(3, 4, Terrain::Sand);
        assert_eq!(world.terrain(3, 4), Some(Terrain::Sand));
        // The rest of its chunk is still void.
        assert_eq!(world.terrain(4, 4), None);
        world.clear_tile(3, 4);
        assert_eq!(world.terrain(3, 4), None);
    }

    #[test]
    fn negative_tiles_land_in_their_own_chunks() {
        let mut world = World::new();
        world.set_terrain(-1, -1, Terrain::Water);
        world.set_terrain(0, 0, Terrain::Dirt);
        world.set_terrain(-CHUNK, -CHUNK, Terrain::Sand);
        assert_eq!(world.terrain(-1, -1), Some(Terrain::Water));
        assert_eq!(world.terrain(0, 0), Some(Terrain::Dirt));
        assert_eq!(world.terrain(-CHUNK, -CHUNK), Some(Terrain::Sand));
        assert_eq!(world.terrain(-CHUNK - 1, -CHUNK), None);
        assert_eq!(world.bounds(), Some([-CHUNK, -CHUNK, CHUNK, CHUNK]));
    }

    #[test]
    fn one_prop_per_tile_and_only_on_ground() {
        let mut world = World::new();
        assert!(!world.place_prop(PropKind::Tree, 5, 5), "no ground");
        world.set_terrain(5, 5, Terrain::Grass);
        assert!(world.place_prop(PropKind::Tree, 5, 5));
        assert!(!world.place_prop(PropKind::Rock, 5, 5), "taken");
        assert_eq!(world.prop_at(5, 5).map(|p| p.kind), Some(PropKind::Tree));
        assert_eq!(world.remove_prop(5, 5).map(|p| p.kind), Some(PropKind::Tree));
        assert_eq!(world.prop_at(5, 5), None);
    }

    #[test]
    fn clearing_a_tile_takes_its_prop() {
        let mut world = World::new();
        world.set_terrain(1, 1, Terrain::Grass);
        world.place_prop(PropKind::Rock, 1, 1);
        world.clear_tile(1, 1);
        assert_eq!(world.prop_at(1, 1), None);
    }

    #[test]
    fn props_in_a_box_cross_chunks_and_respect_its_edges() {
        let mut world = World::new();
        let spots = [(-40, 0), (-1, -1), (0, 0), (31, 31), (32, 32), (70, 10)];
        for &(x, y) in &spots {
            world.set_terrain(x, y, Terrain::Grass);
            assert!(world.place_prop(PropKind::Tree, x, y));
        }
        let mut found: Vec<(i32, i32)> = world.props_in(-1, -1, 32, 32).map(|p| (p.x, p.y)).collect();
        found.sort();
        assert_eq!(found, vec![(-1, -1), (0, 0), (31, 31)]);
        assert_eq!(world.props_in(-100, -100, 100, 100).count(), spots.len());
        assert_eq!(world.props_in(5, 5, 6, 6).count(), 0);
    }

    #[test]
    fn an_empty_world_has_no_bounds() {
        assert_eq!(World::new().bounds(), None);
    }

    #[test]
    fn set_tile_makes_the_tile_exactly_what_was_asked() {
        let mut world = World::new();
        let tree_on_grass = Tile { terrain: Some(Terrain::Grass), prop: Some(PropKind::Tree) };
        world.set_tile(2, 2, tree_on_grass);
        assert_eq!(world.tile(2, 2), tree_on_grass);
        // New ground under the same prop, then another prop, then none.
        for tile in [Tile { terrain: Some(Terrain::Sand), prop: Some(PropKind::Tree) }, Tile { terrain: Some(Terrain::Sand), prop: Some(PropKind::Rock) }, Tile { terrain: Some(Terrain::Sand), prop: None }] {
            world.set_tile(2, 2, tile);
            assert_eq!(world.tile(2, 2), tile);
        }
        // No ground: nothing can stand there.
        world.set_tile(2, 2, Tile { terrain: None, prop: Some(PropKind::Tree) });
        assert_eq!(world.tile(2, 2), Tile::default());
    }

    #[test]
    fn every_change_marks_the_world_dirty_and_looking_does_not() {
        let mut world = World::new();
        assert!(!world.is_dirty());
        let _ = (world.terrain(0, 0), world.tile(0, 0), world.prop_at(0, 0), world.bounds());
        world.clear_tile(0, 0);
        assert!(!world.is_dirty(), "clearing what was never there changes nothing");
        world.set_terrain(0, 0, Terrain::Grass);
        assert!(world.is_dirty());
        let changes: [fn(&mut World); 3] = [
            |w| {
                w.place_prop(PropKind::Tree, 0, 0);
            },
            |w| {
                w.remove_prop(0, 0);
            },
            |w| w.clear_tile(0, 0),
        ];
        for change in changes {
            world.dirty.clear();
            change(&mut world);
            assert!(world.is_dirty());
        }
    }

    #[test]
    fn prop_names_round_trip() {
        for kind in PropKind::ALL {
            assert_eq!(PropKind::from_name(kind.name()), Some(kind));
        }
        assert_eq!(PropKind::from_name("dragon"), None);
    }
}
