//! The world: tiles kept in chunks, and the things standing on them.
//!
//! Tile `(x, y)` covers the square from `(x, y)` to `(x + 1, y + 1)`; x
//! grows east and y grows south. A tile that was never set is void:
//! there is no ground there at all.

use std::collections::HashMap;

use crate::terrain::Terrain;

/// Tiles along one side of a chunk.
pub const CHUNK: i32 = 32;
const AREA: usize = (CHUNK * CHUNK) as usize;
const VOID: u8 = u8::MAX;

/// What a prop is. Placeholders for the test map: objects get their
/// real definitions from data once the editor exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropKind {
    Tree,
    Rock,
    HerbPurple,
    HerbCyan,
}

/// Something standing on a tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Prop {
    pub kind: PropKind,
    pub x: i32,
    pub y: i32,
}

struct Chunk {
    /// A terrain's rank per tile, row by row, or [`VOID`].
    tiles: Box<[u8; AREA]>,
    props: Vec<Prop>,
}

impl Chunk {
    fn new() -> Chunk {
        Chunk { tiles: Box::new([VOID; AREA]), props: Vec::new() }
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
    chunks: HashMap<(i32, i32), Chunk>,
}

impl World {
    pub fn new() -> World {
        World::default()
    }

    /// The ground at a tile; `None` where there is none.
    pub fn terrain(&self, x: i32, y: i32) -> Option<Terrain> {
        let (key, index) = locate(x, y);
        Terrain::from_rank(self.chunks.get(&key)?.tiles[index])
    }

    pub fn set_terrain(&mut self, x: i32, y: i32, terrain: Terrain) {
        let (key, index) = locate(x, y);
        self.chunks.entry(key).or_insert_with(Chunk::new).tiles[index] = terrain.rank();
    }

    /// Take a tile's ground away, and whatever stood on it.
    pub fn clear_tile(&mut self, x: i32, y: i32) {
        let (key, index) = locate(x, y);
        if let Some(chunk) = self.chunks.get_mut(&key) {
            chunk.tiles[index] = VOID;
            chunk.props.retain(|p| (p.x, p.y) != (x, y));
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
        true
    }

    pub fn remove_prop(&mut self, x: i32, y: i32) -> Option<Prop> {
        let (key, _) = locate(x, y);
        let props = &mut self.chunks.get_mut(&key)?.props;
        let at = props.iter().position(|p| (p.x, p.y) == (x, y))?;
        Some(props.swap_remove(at))
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
}
