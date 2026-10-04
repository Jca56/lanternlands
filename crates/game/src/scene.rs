//! What one frame draws, worked out from the world and the camera.
//!
//! Ground is drawn on the dual grid: one quad per square between four
//! tile centres, shaped by which of those four tiles it covers. Where
//! two kinds of ground meet, the one that ranks higher lies over the
//! other with rounded corners, so nothing is ever a bare square.

use lntrn_core::impl_pod;
use lntrn_math::Vec2;
use sim::noise::unit;
use sim::{PropKind, Terrain, World};

use crate::assets::{SpriteId, SpriteSet};
use crate::camera::Camera;

/// How many grass tiles in a hundred grow a tuft, and how far from the
/// middle of its tile one may stand (well inside the round a lone tile
/// keeps when other ground closes in).
const TUFT_CHANCE: f64 = 0.22;
const TUFT_REACH: f64 = 0.25;
const TUFT_SALT: u64 = 0x7f;

/// One layer of ground in one dual tile.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct TerrainInstance {
    /// The dual tile, counted from the scene's origin.
    pub tile: [i32; 2],
    pub mask: u32,
    /// The terrain's rank.
    pub terrain: u32,
}
impl_pod!(unsafe TerrainInstance);

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct SpriteInstance {
    /// Its top-left corner in tiles, counted from the scene's origin.
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv: [f32; 4],
    /// Multiplies the sprite; premultiplied like it.
    pub tint: [f32; 4],
}
impl_pod!(unsafe SpriteInstance);

/// The layers of ground in a dual tile whose corners sit on `corners`
/// (top-left, top-right, bottom-left, bottom-right; `None` where there
/// is no tile), bottom first, each with a mask of the corners it covers:
/// bits 1, 2, 4 and 8 in that order. A layer covers its own tiles and
/// those of every kind that lies over it, so no gap opens where two
/// kinds meet.
pub fn layers(corners: [Option<Terrain>; 4]) -> impl Iterator<Item = (Terrain, u8)> {
    Terrain::ALL.into_iter().filter(move |t| corners.contains(&Some(*t))).map(move |t| {
        let mask = corners.iter().enumerate().filter(|(_, c)| c.is_some_and(|c| c >= t)).fold(0, |mask, (i, _)| mask | 1 << i);
        (t, mask)
    })
}

/// Which sprite each kind of prop is drawn with.
#[derive(Clone, Copy, Debug, Default)]
pub struct Look {
    tree: Option<SpriteId>,
    rock: Option<SpriteId>,
    herb_purple: Option<SpriteId>,
    herb_cyan: Option<SpriteId>,
    tuft: Option<SpriteId>,
}

impl Look {
    pub fn find(set: &SpriteSet) -> Look {
        Look { tree: set.id("tree"), rock: set.id("rock"), herb_purple: set.id("herb_purple"), herb_cyan: set.id("herb_cyan"), tuft: set.id("tuft") }
    }

    /// The names of the sprites it looked for and did not find.
    pub fn missing(&self) -> Vec<&'static str> {
        let all = [("tree", self.tree), ("rock", self.rock), ("herb_purple", self.herb_purple), ("herb_cyan", self.herb_cyan), ("tuft", self.tuft)];
        all.into_iter().filter(|(_, id)| id.is_none()).map(|(name, _)| name).collect()
    }

    /// The sprite a kind of prop is drawn with, if the assets have it.
    pub fn prop(&self, kind: PropKind) -> Option<SpriteId> {
        match kind {
            PropKind::Tree => self.tree,
            PropKind::Rock => self.rock,
            PropKind::HerbPurple => self.herb_purple,
            PropKind::HerbCyan => self.herb_cyan,
        }
    }
}

#[derive(Default)]
pub struct Scene {
    /// The tile every instance's position is counted from: the one the
    /// camera is over, so the numbers stay small and exact.
    pub origin: [i32; 2],
    /// From the camera's centre to `origin`, in tiles.
    pub offset: [f32; 2],
    /// Physical pixels a tile is across.
    pub tile_px: f32,
    pub terrain: Vec<TerrainInstance>,
    /// Back to front.
    pub sprites: Vec<SpriteInstance>,
    /// Each sprite with the ground point it is sorted by.
    standing: Vec<(Vec2, SpriteInstance)>,
}

impl Scene {
    /// Everything a view of `size` pixels shows.
    pub fn build(&mut self, world: &World, camera: &Camera, size: Vec2, scale: f64, set: &SpriteSet, look: &Look) {
        let seen = camera.visible(size, scale);
        let origin = camera.center.floor();
        self.origin = [origin.x as i32, origin.y as i32];
        self.offset = (origin - camera.center).to_gpu();
        self.tile_px = camera.tile_px(scale) as f32;

        self.terrain.clear();
        // Dual tile (i, j) spans half a tile either side of the corner
        // the four tiles around it share.
        let (i0, i1) = ((seen.min.x + 0.5).floor() as i32, (seen.max.x + 0.5).floor() as i32);
        let (j0, j1) = ((seen.min.y + 0.5).floor() as i32, (seen.max.y + 0.5).floor() as i32);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let corners = [world.terrain(i - 1, j - 1), world.terrain(i, j - 1), world.terrain(i - 1, j), world.terrain(i, j)];
                let tile = [i - self.origin[0], j - self.origin[1]];
                self.terrain.extend(layers(corners).map(|(t, mask)| TerrainInstance { tile, mask: mask as u32, terrain: t.rank() as u32 }));
            }
        }

        self.standing.clear();
        // Sprites reach past their own tile: up, mostly, and a little
        // to each side.
        let (x0, x1) = (seen.min.x.floor() as i32 - 2, seen.max.x.floor() as i32 + 3);
        let (y0, y1) = (seen.min.y.floor() as i32 - 1, seen.max.y.floor() as i32 + 4);
        for prop in world.props_in(x0, y0, x1, y1) {
            if let Some(id) = look.prop(prop.kind) {
                let ground = Vec2::new(prop.x as f64 + 0.5, prop.y as f64 + 0.5);
                self.stand(set, id, ground, origin);
            }
        }
        if let Some(id) = look.tuft {
            for y in y0..y1 {
                for x in x0..x1 {
                    if let Some(ground) = tuft(world, x, y) {
                        self.stand(set, id, ground, origin);
                    }
                }
            }
        }
        // What stands further down the screen is in front.
        self.standing.sort_by(|(a, _), (b, _)| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
        self.sprites.clear();
        self.sprites.extend(self.standing.iter().map(|(_, sprite)| *sprite));
    }

    fn stand(&mut self, set: &SpriteSet, id: SpriteId, ground: Vec2, origin: Vec2) {
        let sprite = set.get(id);
        let instance = SpriteInstance { pos: (ground - sprite.anchor - origin).to_gpu(), size: sprite.size.to_gpu(), uv: sprite.uv, tint: [1.0; 4] };
        self.standing.push((ground, instance));
    }
}

/// Where the tuft on a tile grows, if it has one: some bare grass tiles do.
fn tuft(world: &World, x: i32, y: i32) -> Option<Vec2> {
    let bare = world.terrain(x, y) == Some(Terrain::Grass) && world.prop_at(x, y).is_none();
    (bare && unit(x, y, TUFT_SALT) < TUFT_CHANCE).then(|| {
        let nudge = |salt| (unit(x, y, salt) - 0.5) * 2.0 * TUFT_REACH;
        Vec2::new(x as f64 + 0.5 + nudge(TUFT_SALT + 1), y as f64 + 0.5 + nudge(TUFT_SALT + 2))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Terrain::{Dirt, Grass, Sand, Water};

    const TL: u8 = 1;
    const TR: u8 = 2;
    const BL: u8 = 4;
    const BR: u8 = 8;
    const FULL: u8 = TL | TR | BL | BR;

    fn of(corners: [Option<Terrain>; 4]) -> Vec<(Terrain, u8)> {
        layers(corners).collect()
    }

    #[test]
    fn one_kind_all_round_is_one_full_layer() {
        assert_eq!(of([Some(Water); 4]), vec![(Water, FULL)]);
        assert_eq!(of([None; 4]), vec![]);
    }

    #[test]
    fn the_higher_kind_lies_over_a_full_layer_of_the_lower() {
        // A path's corner on grass: grass everywhere, dirt on its one tile.
        assert_eq!(of([Some(Grass), Some(Grass), Some(Grass), Some(Dirt)]), vec![(Grass, FULL), (Dirt, BR)]);
        assert_eq!(of([Some(Dirt), Some(Dirt), Some(Grass), Some(Grass)]), vec![(Grass, FULL), (Dirt, TL | TR)]);
    }

    #[test]
    fn a_layer_runs_under_what_lies_over_it() {
        // Dirt beside sand: the dirt goes on under the sand, so the
        // sand's round corner shows dirt, not a sliver of grass.
        assert_eq!(of([Some(Dirt), Some(Sand), Some(Dirt), Some(Sand)]), vec![(Dirt, FULL), (Sand, TR | BR)]);
        assert_eq!(of([Some(Grass), Some(Dirt), Some(Sand), Some(Water)]), vec![(Grass, FULL), (Dirt, TR | BL | BR), (Sand, BL | BR), (Water, BR)]);
    }

    #[test]
    fn the_worlds_edge_is_a_shape_too() {
        // Two tiles of grass above nothing: the grass is rounded off.
        assert_eq!(of([Some(Grass), Some(Grass), None, None]), vec![(Grass, TL | TR)]);
        assert_eq!(of([None, Some(Water), None, Some(Grass)]), vec![(Grass, TR | BR), (Water, TR)]);
    }

    #[test]
    fn inside_the_map_every_dual_tile_is_fully_covered() {
        let world = sim::testmap::build();
        for j in 1..sim::testmap::HEIGHT {
            for i in 1..sim::testmap::WIDTH {
                let corners = [world.terrain(i - 1, j - 1), world.terrain(i, j - 1), world.terrain(i - 1, j), world.terrain(i, j)];
                assert_eq!(layers(corners).next().map(|(_, mask)| mask), Some(FULL), "dual tile ({i}, {j})");
            }
        }
    }

    fn set() -> SpriteSet {
        SpriteSet::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")).expect("assets load")
    }

    fn built(center: Vec2, size: Vec2) -> Scene {
        let (world, set) = (sim::testmap::build(), set());
        let mut scene = Scene::default();
        scene.build(&world, &Camera::new(center), size, 1.0, &set, &Look::find(&set));
        scene
    }

    #[test]
    fn every_sprite_the_scene_asks_for_is_in_the_assets() {
        assert_eq!(Look::find(&set()).missing(), Vec::<&str>::new());
        assert_eq!(Look::find(&SpriteSet::empty()).missing().len(), 5);
    }

    #[test]
    fn instances_are_counted_from_the_tile_under_the_camera() {
        let scene = built(Vec2::new(20.25, 10.75), Vec2::new(800.0, 600.0));
        assert_eq!(scene.origin, [20, 10]);
        assert_eq!(scene.offset, [-0.25, -0.75]);
        assert_eq!(scene.tile_px, 100.0);
        // 8 by 6 tiles are in view, so nothing is far from the origin.
        assert!(scene.terrain.iter().all(|t| t.tile[0].abs() <= 5 && t.tile[1].abs() <= 4), "a dual tile out of view");
        // And the dual tile right under the camera is there.
        assert!(scene.terrain.iter().any(|t| t.tile == [0, 1]));
    }

    #[test]
    fn the_view_is_covered_edge_to_edge() {
        let scene = built(Vec2::new(20.25, 10.75), Vec2::new(800.0, 600.0));
        // The view spans x 16.25..24.25 and y 7.75..13.75: every dual
        // tile touching it has a layer.
        for j in 8..=14 {
            for i in 16..=24 {
                let tile = [i - scene.origin[0], j - scene.origin[1]];
                assert!(scene.terrain.iter().any(|t| t.tile == tile), "no ground drawn at dual tile ({i}, {j})");
            }
        }
    }

    #[test]
    fn sprites_are_sorted_back_to_front() {
        let scene = built(Vec2::new(40.0, 26.0), Vec2::new(1600.0, 1000.0));
        assert!(scene.sprites.len() > 20, "the woods are in view: {} sprites", scene.sprites.len());
        let feet: Vec<f64> = scene.standing.iter().map(|(ground, _)| ground.y).collect();
        assert!(feet.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(scene.sprites.len(), scene.standing.len());
    }

    #[test]
    fn a_prop_stands_with_its_anchor_on_the_middle_of_its_tile() {
        let (world, set) = (sim::testmap::build(), set());
        let tree = world.props_in(0, 0, sim::testmap::WIDTH, sim::testmap::HEIGHT).find(|p| p.kind == PropKind::Tree).expect("a tree");
        let center = Vec2::new(tree.x as f64 + 0.5, tree.y as f64 + 0.5);
        let mut scene = Scene::default();
        let look = Look::find(&set);
        scene.build(&world, &Camera::new(center), Vec2::new(400.0, 400.0), 1.0, &set, &look);
        let sprite = set.get(look.tree.unwrap());
        let want = (center - sprite.anchor - Vec2::new(tree.x as f64, tree.y as f64)).to_gpu();
        assert!(scene.sprites.iter().any(|s| s.pos == want && s.uv == sprite.uv), "the tree is not where its anchor puts it");
    }

    #[test]
    fn tufts_grow_only_on_bare_grass_and_stay_near_the_middle() {
        let world = sim::testmap::build();
        let mut count = 0;
        for y in 0..sim::testmap::HEIGHT {
            for x in 0..sim::testmap::WIDTH {
                let Some(at) = tuft(&world, x, y) else { continue };
                count += 1;
                assert_eq!(world.terrain(x, y), Some(Terrain::Grass));
                assert_eq!(world.prop_at(x, y), None);
                let from_middle = at - Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
                assert!(from_middle.x.abs() <= TUFT_REACH && from_middle.y.abs() <= TUFT_REACH);
            }
        }
        assert!(count > 100, "{count} tufts on the whole map");
    }
}
