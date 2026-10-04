//! The world: tiles kept in chunks, and the things standing on them.
//!
//! Tile `(x, y)` covers the square from `(x, y)` to `(x + 1, y + 1)`; x
//! grows east and y grows south. A tile that was never set is void:
//! there is no ground there at all.

use std::collections::{BTreeSet, HashMap};

use crate::monsters::MonsterKind;
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
    TinRock,
    Furnace,
    Anvil,
}

impl PropKind {
    pub const ALL: [PropKind; 7] = [PropKind::Tree, PropKind::Rock, PropKind::TinRock, PropKind::HerbPurple, PropKind::HerbCyan, PropKind::Furnace, PropKind::Anvil];

    /// What it is called in a map file. Never change one: saved maps
    /// hold these.
    pub const fn name(self) -> &'static str {
        match self {
            PropKind::Tree => "tree",
            PropKind::Rock => "rock",
            PropKind::HerbPurple => "herb_purple",
            PropKind::HerbCyan => "herb_cyan",
            PropKind::TinRock => "tin_rock",
            PropKind::Furnace => "furnace",
            PropKind::Anvil => "anvil",
        }
    }

    pub fn from_name(name: &str) -> Option<PropKind> {
        PropKind::ALL.into_iter().find(|k| k.name() == name)
    }

    /// Whether it is in the way of someone walking.
    pub const fn blocks(self) -> bool {
        !matches!(self, PropKind::HerbPurple | PropKind::HerbCyan)
    }
}

/// A place the map marks out. There is at most one of each in a world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Marker {
    /// Where the player appears.
    PlayerStart,
}

impl Marker {
    pub const ALL: [Marker; 1] = [Marker::PlayerStart];

    /// What it is called in a map file. Never change one.
    pub const fn name(self) -> &'static str {
        match self {
            Marker::PlayerStart => "player_start",
        }
    }

    pub fn from_name(name: &str) -> Option<Marker> {
        Marker::ALL.into_iter().find(|m| m.name() == name)
    }
}

/// Something standing on a tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Prop {
    pub kind: PropKind,
    pub x: i32,
    pub y: i32,
}

/// Everything about one tile: its ground, what stands on it, and what
/// the map marks there. No ground means none of the rest either.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tile {
    pub terrain: Option<Terrain>,
    pub prop: Option<PropKind>,
    pub marker: Option<Marker>,
    /// The kind of monster that lives here.
    pub spawn: Option<MonsterKind>,
}

pub(crate) struct Chunk {
    /// A terrain's rank per tile, row by row, or [`VOID`].
    pub(crate) tiles: Box<[u8; AREA]>,
    pub(crate) props: Vec<Prop>,
    /// Each marker in this chunk, with its tile.
    pub(crate) markers: Vec<(Marker, i32, i32)>,
    /// Each tile in this chunk where a monster lives, and its kind.
    pub(crate) spawns: Vec<(MonsterKind, i32, i32)>,
}

impl Chunk {
    pub(crate) fn new() -> Chunk {
        Chunk { tiles: Box::new([VOID; AREA]), props: Vec::new(), markers: Vec::new(), spawns: Vec::new() }
    }

    /// No ground anywhere and nothing standing: not worth keeping.
    pub(crate) fn is_empty(&self) -> bool {
        self.props.is_empty() && self.markers.is_empty() && self.spawns.is_empty() && self.tiles.iter().all(|&t| t == VOID)
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
            chunk.markers.retain(|&(_, mx, my)| (mx, my) != (x, y));
            chunk.spawns.retain(|&(_, sx, sy)| (sx, sy) != (x, y));
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

    /// Where a marker is, if the map has it.
    pub fn marker(&self, marker: Marker) -> Option<(i32, i32)> {
        self.chunks.values().flat_map(|chunk| &chunk.markers).find(|(m, _, _)| *m == marker).map(|&(_, x, y)| (x, y))
    }

    /// What the map marks on a tile.
    pub fn marker_at(&self, x: i32, y: i32) -> Option<Marker> {
        let (key, _) = locate(x, y);
        self.chunks.get(&key)?.markers.iter().find(|&&(_, mx, my)| (mx, my) == (x, y)).map(|&(m, _, _)| m)
    }

    /// Put a marker on a tile, taking it from wherever it was (and
    /// taking away any other marker on that tile). `false`, and nothing
    /// changes, when the tile has no ground.
    pub fn set_marker(&mut self, marker: Marker, x: i32, y: i32) -> bool {
        if self.terrain(x, y).is_none() {
            return false;
        }
        self.take_marker(marker);
        self.clear_marker(x, y);
        let (key, _) = locate(x, y);
        self.chunks.get_mut(&key).expect("the tile has ground").markers.push((marker, x, y));
        self.dirty.insert(key);
        true
    }

    /// Take a marker off the map, wherever it is.
    pub fn take_marker(&mut self, marker: Marker) {
        if let Some((x, y)) = self.marker(marker) {
            self.clear_marker(x, y);
        }
    }

    /// Take away whatever the map marks on a tile.
    pub fn clear_marker(&mut self, x: i32, y: i32) {
        let (key, _) = locate(x, y);
        if let Some(chunk) = self.chunks.get_mut(&key).filter(|c| c.markers.iter().any(|&(_, mx, my)| (mx, my) == (x, y))) {
            chunk.markers.retain(|&(_, mx, my)| (mx, my) != (x, y));
            self.dirty.insert(key);
        }
    }

    /// The kind of monster that lives on a tile.
    pub fn spawn_at(&self, x: i32, y: i32) -> Option<MonsterKind> {
        let (key, _) = locate(x, y);
        self.chunks.get(&key)?.spawns.iter().find(|&&(_, sx, sy)| (sx, sy) == (x, y)).map(|&(kind, _, _)| kind)
    }

    /// Make a tile the home of a kind of monster, or of none. `false`,
    /// and nothing changes, when the tile has no ground.
    pub fn set_spawn(&mut self, x: i32, y: i32, kind: Option<MonsterKind>) -> bool {
        if self.terrain(x, y).is_none() {
            return false;
        }
        if self.spawn_at(x, y) != kind {
            let (key, _) = locate(x, y);
            let spawns = &mut self.chunks.get_mut(&key).expect("the tile has ground").spawns;
            spawns.retain(|&(_, sx, sy)| (sx, sy) != (x, y));
            spawns.extend(kind.map(|kind| (kind, x, y)));
            self.dirty.insert(key);
        }
        true
    }

    /// Every monster's home on the map, row by row.
    pub fn spawns(&self) -> Vec<(MonsterKind, i32, i32)> {
        let mut all: Vec<_> = self.chunks.values().flat_map(|chunk| chunk.spawns.iter().copied()).collect();
        all.sort_by_key(|&(_, x, y)| (y, x));
        all
    }

    /// Whether someone can stand on a tile: ground that can be walked
    /// on, with nothing in the way.
    pub fn walkable(&self, x: i32, y: i32) -> bool {
        self.terrain(x, y).is_some_and(Terrain::walkable) && !self.prop_at(x, y).is_some_and(|p| p.kind.blocks())
    }

    /// Everything about a tile at once.
    pub fn tile(&self, x: i32, y: i32) -> Tile {
        Tile { terrain: self.terrain(x, y), prop: self.prop_at(x, y).map(|p| p.kind), marker: self.marker_at(x, y), spawn: self.spawn_at(x, y) }
    }

    /// Make a tile exactly `tile`. With no ground, nothing stands on it
    /// and nothing is marked there, whatever the rest of `tile` says. A
    /// marker put here is taken from wherever else it was.
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
        if self.marker_at(x, y) != tile.marker {
            match tile.marker {
                Some(marker) => {
                    self.set_marker(marker, x, y);
                }
                None => self.clear_marker(x, y),
            }
        }
        self.set_spawn(x, y, tile.spawn);
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
#[path = "world_tests.rs"]
mod tests;
