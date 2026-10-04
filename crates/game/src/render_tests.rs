//! The renderer measured against the art: the test map is drawn with no
//! window into a picture, and that picture's pixels are compared with
//! the palette and with colours read off the approved style sheet.
//!
//! Set `EXPLORE_RENDER_OUT` to a folder to keep each picture as a PNG.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

use lntrn_image::Image;
use lntrn_math::{Rect, Vec2};
use lntrn_render::Readback;
use sim::{PropKind, World};

use super::*;
use crate::assets::SpriteSet;
use crate::camera::Camera;
use crate::scene::{Look, Placed};

const SIZE: [u32; 2] = [800, 600];
const VIEW: Rect = Rect::new(Vec2::ZERO, Vec2::new(SIZE[0] as f64, SIZE[1] as f64));

/// Colours read off `art/concepts/style-test-01` (panel B) in Lantern
/// Studio: what the approved art looks like.
const STUDIO_GRASS: u32 = 0x73a14c;
const STUDIO_TRUNK: u32 = 0x785230;
const STUDIO_FOLIAGE: u32 = 0x4b7f39;
const STUDIO_SHADOW_ON_GRASS: u32 = 0x5a7b38;

struct Shot {
    camera: Camera,
    world: World,
    /// The world picture, and the same after it was put on an sRGB window.
    picture: Image,
    window: Image,
}

fn target(gpu: &Gpu, format: wgpu::TextureFormat) -> wgpu::Texture {
    gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test target"),
        size: wgpu::Extent3d { width: SIZE[0], height: SIZE[1], depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// One device for every test here, and one test on it at a time. The
/// test run segfaulted once with each test opening a device of its own,
/// a dozen at once from as many threads; it has not with this.
pub(crate) fn gpu() -> Option<(&'static Gpu, MutexGuard<'static, ()>)> {
    static GPU: OnceLock<Option<Gpu>> = OnceLock::new();
    static TURN: Mutex<()> = Mutex::new(());
    let turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let gpu = GPU.get_or_init(|| Gpu::new(None).map_err(|e| eprintln!("no GPU: {e}")).ok());
    gpu.as_ref().map(|gpu| (gpu, turn))
}

/// Draw the test map as the game does, looking at `center`. `None`, and
/// a line saying so, on a machine with no GPU.
fn shoot(name: &str, center: Vec2, zoom: f64) -> Option<Shot> {
    shoot_with(name, center, zoom, true)
}

/// The same, with or without the sprites.
fn shoot_with(name: &str, center: Vec2, zoom: f64, sprites: bool) -> Option<Shot> {
    shoot_scene(name, center, zoom, sprites, |_| Vec::new())
}

/// The same, with whatever `extras` puts in the world besides the map.
fn shoot_scene(name: &str, center: Vec2, zoom: f64, sprites: bool, extras: impl FnOnce(&SpriteSet) -> Vec<Placed>) -> Option<Shot> {
    shoot_all(name, center, zoom, sprites, extras, &|_, _| false)
}

/// The same, with `spent` saying which tiles' nodes have given all they had.
fn shoot_all(name: &str, center: Vec2, zoom: f64, sprites: bool, extras: impl FnOnce(&SpriteSet) -> Vec<Placed>, spent: &dyn Fn(i32, i32) -> bool) -> Option<Shot> {
    let Some((gpu, _turn)) = gpu() else {
        eprintln!("SKIPPED {name}: no GPU to draw with");
        return None;
    };
    let set = SpriteSet::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")).expect("assets load");
    let world = sim::testmap::build();
    let camera = Camera { center, zoom };
    let mut scene = Scene::default();
    let look = if sprites { Look::find(&set) } else { Look::default() };
    scene.build(&world, &camera, VIEW.size(), 1.0, &set, &look, &extras(&set), spent);

    let window_format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut renderer = WorldRenderer::new(gpu, window_format, &set.sheet);
    renderer.prepare(gpu, &scene, SIZE);
    let (picture, window) = (target(gpu, WorldRenderer::TARGET_FORMAT), target(gpu, window_format));
    let (picture_view, window_view) = (picture.create_view(&Default::default()), window.create_view(&Default::default()));
    let mut encoder = gpu.create_encoder("test frame");
    renderer.draw(&mut encoder, &picture_view);
    renderer.composite(gpu, &mut encoder, &picture_view, &window_view, [0, 0, SIZE[0], SIZE[1]]);
    let reads = [Readback::begin(gpu, &mut encoder, &picture, true).expect("readable"), Readback::begin(gpu, &mut encoder, &window, true).expect("readable")];
    gpu.queue.submit([encoder.finish()]);
    let [picture, window] = reads.map(|read| read.finish(gpu).expect("the pixels come back"));

    if let Some(dir) = std::env::var_os("EXPLORE_RENDER_OUT") {
        let path = Path::new(&dir).join(format!("{name}.png"));
        std::fs::write(&path, lntrn_image::encode_png(&picture)).expect("the picture is written");
    }
    Some(Shot { camera, world, picture, window })
}

impl Shot {
    /// The colour drawn at a point of the world.
    fn at(&self, world: Vec2) -> [u8; 3] {
        let p = self.camera.to_screen(world, VIEW, 1.0);
        assert!(VIEW.contains(p), "{world:?} is not in the picture");
        let [r, g, b, _] = self.picture.pixel(p.x as u32, p.y as u32);
        [r, g, b]
    }

    /// The first tile for which `fits` holds with nothing standing near
    /// enough to be drawn over it, among those well inside the picture.
    fn quiet_tile(&self, fits: impl Fn(i32, i32) -> bool) -> (i32, i32) {
        let seen = self.camera.visible(VIEW.size(), 1.0).shrink(1.0);
        let (x0, x1, y0, y1) = (seen.min.x.ceil() as i32, seen.max.x.floor() as i32, seen.min.y.ceil() as i32, seen.max.y.floor() as i32);
        (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).find(|&(x, y)| fits(x, y) && self.world.props_in(x - 2, y - 1, x + 3, y + 4).all(|p| (p.x, p.y) == (x, y))).expect("a quiet tile in view")
    }
}

fn bytes_of_hex(hex: u32) -> [u8; 3] {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
}

fn close(got: [u8; 3], want: u32, tolerance: u8, what: &str) {
    let want = bytes_of_hex(want);
    let off = got.iter().zip(&want).map(|(g, w)| g.abs_diff(*w)).max().unwrap();
    assert!(off <= tolerance, "{what}: drew #{:02x}{:02x}{:02x}, wanted #{:02x}{:02x}{:02x}", got[0], got[1], got[2], want[0], want[1], want[2]);
}

fn fill_hex(terrain: Terrain) -> u32 {
    let [r, g, b] = terrain_style(terrain).fill.map(|c| (c * 255.0).round() as u32);
    r << 16 | g << 8 | b
}

#[test]
fn each_ground_is_its_palette_colour_at_the_middle_of_a_tile() {
    // The clearing for dirt, the big pond's west shore for sand and water.
    for (name, terrain, center) in [("dirt", Terrain::Dirt, Vec2::new(24.5, 14.5)), ("sand", Terrain::Sand, Vec2::new(29.0, 21.0)), ("water", Terrain::Water, Vec2::new(29.0, 21.0))] {
        let Some(shot) = shoot(name, center, 1.0) else { return };
        let (x, y) = shot.quiet_tile(|x, y| shot.world.terrain(x, y) == Some(terrain));
        close(shot.at(Vec2::new(x as f64 + 0.5, y as f64 + 0.5)), fill_hex(terrain), 1, &format!("{terrain:?} at tile ({x}, {y})"));
    }
    // Grass is measured where four grass tiles meet: no tuft reaches there.
    let Some(shot) = shoot("grass", Vec2::new(16.0, 5.0), 1.0) else { return };
    let grass = |x, y| shot.world.terrain(x, y) == Some(Terrain::Grass);
    let (x, y) = shot.quiet_tile(|x, y| grass(x, y) && grass(x - 1, y) && grass(x, y - 1) && grass(x - 1, y - 1) && shot.world.prop_at(x, y).is_none());
    close(shot.at(Vec2::new(x as f64, y as f64)), STUDIO_GRASS, 1, &format!("grass at the corner of tile ({x}, {y})"));
}

#[test]
fn a_path_has_its_rim_inside_its_edge_and_grass_outside() {
    let Some(shot) = shoot("rim", Vec2::new(24.5, 15.5), 1.0) else { return };
    let is = |x, y, t| shot.world.terrain(x, y) == Some(t);
    // Two dirt tiles side by side over two grass tiles: a straight edge
    // along the line between the rows.
    let (x, y) = shot.quiet_tile(|x, y| is(x, y, Terrain::Dirt) && is(x + 1, y, Terrain::Dirt) && is(x, y + 1, Terrain::Grass) && is(x + 1, y + 1, Terrain::Grass) && shot.world.props_in(x - 2, y, x + 4, y + 5).count() == 0);
    let (edge_x, edge_y) = (x as f64 + 1.0, y as f64 + 1.0);
    let rim = terrain_style(Terrain::Dirt);
    let rim_hex = { let [r, g, b] = rim.rim.map(|c| (c * 255.0).round() as u32); r << 16 | g << 8 | b };
    let where_ = format!("the path's edge under tiles ({x}, {y}) and ({}, {y})", x + 1);
    close(shot.at(Vec2::new(edge_x, edge_y - 0.3)), fill_hex(Terrain::Dirt), 1, &format!("{where_}: well inside"));
    close(shot.at(Vec2::new(edge_x, edge_y - rim.rim_width as f64 / 2.0)), rim_hex, 1, &format!("{where_}: in the rim"));
    close(shot.at(Vec2::new(edge_x, edge_y + 0.08)), STUDIO_GRASS, 1, &format!("{where_}: just outside"));
}

#[test]
fn outside_the_map_is_the_void() {
    let Some(shot) = shoot("void", Vec2::new(1.0, 1.0), 1.0) else { return };
    let [r, g, b] = VOID.map(|c| (c * 255.0).round() as u32);
    close(shot.at(Vec2::new(-2.0, -1.5)), r << 16 | g << 8 | b, 1, "past the map's corner");
    // The grass ends in a round corner: its very corner is void too.
    close(shot.at(Vec2::new(0.05, 0.05)), r << 16 | g << 8 | b, 1, "the map's rounded corner");
    close(shot.at(Vec2::new(1.0, 1.0)), STUDIO_GRASS, 1, "just inside the map");
}

/// A tree on open grass with nothing else standing where it could be
/// drawn over the points the tests measure.
fn lone_tree() -> (i32, i32) {
    let world = sim::testmap::build();
    let (w, h) = (sim::testmap::WIDTH, sim::testmap::HEIGHT);
    let grass = |x, y| world.terrain(x, y) == Some(Terrain::Grass);
    let alone = |x, y| world.props_in(x - 2, y - 1, x + 4, y + 4).count() == 1 && (-1..=1).all(|dy| (-1..=2).all(|dx| grass(x + dx, y + dy)));
    let tree = world.props_in(4, 4, w - 4, h - 4).filter(|p| p.kind == PropKind::Tree).find(|p| alone(p.x, p.y));
    let tree = tree.expect("a tree standing alone on grass");
    (tree.x, tree.y)
}

#[test]
fn a_tree_is_drawn_where_it_stands_in_the_arts_own_colours() {
    let (x, y) = lone_tree();
    let ground = Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
    let Some(shot) = shoot("tree", ground, 1.0) else { return };
    // Points of the tree measured on the style sheet, from its anchor
    // (the foot of the trunk), in tiles: 100 of the sheet's pixels each.
    close(shot.at(ground + Vec2::new(0.0, -0.34)), STUDIO_TRUNK, 2, "the trunk");
    close(shot.at(ground + Vec2::new(0.0, -1.04)), STUDIO_FOLIAGE, 2, "the canopy");
    // The shadow lies over the grass as it did in Studio: blended in the
    // art's own numbers, not in linear light (which would come out
    // near #668d43, visibly paler).
    close(shot.at(ground + Vec2::new(-0.4, 0.01)), STUDIO_SHADOW_ON_GRASS, 6, "the shadow on the grass");
    // And beside the tree there is only grass.
    close(shot.at(ground + Vec2::new(1.5, 0.5)), STUDIO_GRASS, 1, "the grass beside it");
}

#[test]
fn zoomed_out_and_in_the_tree_keeps_its_colours() {
    let (x, y) = lone_tree();
    let ground = Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
    for (name, zoom) in [("tree-far", crate::camera::ZOOM_MIN), ("tree-near", crate::camera::ZOOM_MAX)] {
        let Some(shot) = shoot(name, ground + Vec2::new(0.0, -0.8), zoom) else { return };
        close(shot.at(ground + Vec2::new(0.0, -0.34)), STUDIO_TRUNK, 3, &format!("the trunk at zoom {zoom}"));
        close(shot.at(ground + Vec2::new(0.0, -1.04)), STUDIO_FOLIAGE, 3, &format!("the canopy at zoom {zoom}"));
    }
}

#[test]
fn the_window_shows_the_same_numbers_as_the_world_picture() {
    let Some(shot) = shoot("window", Vec2::new(26.0, 17.5), 1.0) else { return };
    let worst = shot.picture.rgba.iter().zip(&shot.window.rgba).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    assert!(worst <= 1, "a pixel changed by {worst} on its way to the window");
}

#[test]
fn ground_joins_without_a_seam() {
    // The ground alone (sprite outlines are the void's own colour), off
    // the pixel grid and at an odd zoom: a gap between two quads would
    // let the clear colour through.
    for (name, center, zoom) in [("seams-grass", Vec2::new(3.137, 3.291), 0.73), ("seams-shore", Vec2::new(29.413, 20.877), 1.37), ("seams-far", Vec2::new(24.3, 15.9), crate::camera::ZOOM_MIN)] {
        let Some(shot) = shoot_with(name, center, zoom, false) else { return };
        let void = VOID.map(|c| (c * 255.0).round() as u8);
        let mut gaps = 0;
        for py in 0..SIZE[1] {
            for px in 0..SIZE[0] {
                if shot.picture.pixel(px, py)[..3] != void {
                    continue;
                }
                // Outside the map the void is what should be there.
                let world = shot.camera.to_world(Vec2::new(px as f64 + 0.5, py as f64 + 0.5), VIEW, 1.0);
                let inside = world.x > 0.6 && world.y > 0.6 && world.x < sim::testmap::WIDTH as f64 - 0.6 && world.y < sim::testmap::HEIGHT as f64 - 0.6;
                gaps += inside as u32;
            }
        }
        assert_eq!(gaps, 0, "{name}: pixels inside the map where the void shows through");
    }
}

#[test]
fn plain_grass_is_one_flat_colour() {
    // No sprites, mostly grass: every pixel of it is the same.
    let Some(shot) = shoot_with("flat", Vec2::new(16.3, 4.2), 0.73, false) else { return };
    let grass = bytes_of_hex(STUDIO_GRASS);
    let plain = (0..SIZE[1]).flat_map(|y| (0..SIZE[0]).map(move |x| (x, y))).filter(|&(x, y)| shot.picture.pixel(x, y)[..3] == grass).count();
    assert!(plain > (SIZE[0] * SIZE[1] * 2 / 3) as usize, "only {plain} pixels of plain grass");
}

/// Colours of the player's drawing, as it was drawn in Lantern Studio.
const TUNIC: u32 = 0x5572ac;
const SKIN: u32 = 0xe0bd97;
const HAIR: u32 = 0x68411f;
const OUTLINE: u32 = 0x271a1c;

#[test]
fn the_player_is_drawn_on_their_ground_in_their_own_colours() {
    let (x, y) = lone_tree();
    // Two tiles to the tree's right, on the grass the tree test found bare.
    let ground = Vec2::new(x as f64 + 2.0, y as f64 + 1.0);
    let placed = |set: &SpriteSet| vec![Placed { sprite: set.id("player_down_idle").expect("there"), ground, flip: false, tint: [1.0; 4] }];
    let Some(shot) = shoot_scene("player", ground + Vec2::new(0.0, -0.5), 1.0, true, placed) else { return };
    // Points of the drawing from its anchor (between the feet), in
    // tiles: 200 of the sheet's pixels each.
    close(shot.at(ground + Vec2::new(0.0, -0.45)), TUNIC, 2, "the tunic");
    close(shot.at(ground + Vec2::new(0.0, -0.78)), SKIN, 2, "the face, under the eyes");
    close(shot.at(ground + Vec2::new(0.0, -1.18)), HAIR, 2, "the hair");
    close(shot.at(ground + Vec2::new(-0.09, -0.875)), OUTLINE, 3, "the left eye");
    close(shot.at(ground + Vec2::new(0.6, -0.5)), STUDIO_GRASS, 1, "the grass beside them");
}

#[test]
fn the_side_view_faces_right_and_its_mirror_faces_left() {
    let (x, y) = lone_tree();
    let ground = Vec2::new(x as f64 + 2.0, y as f64 + 1.0);
    for (name, flip) in [("player-right", false), ("player-left", true)] {
        let placed = |set: &SpriteSet| vec![Placed { sprite: set.id("player_side_idle").expect("there"), ground, flip, tint: [1.0; 4] }];
        let Some(shot) = shoot_scene(name, ground + Vec2::new(0.0, -0.5), 1.0, true, placed) else { return };
        // The eye is on the side they face; the back of the head is hair.
        let front = if flip { -0.17 } else { 0.17 };
        close(shot.at(ground + Vec2::new(front, -0.875)), OUTLINE, 3, &format!("{name}: the eye"));
        close(shot.at(ground + Vec2::new(-front, -0.875)), HAIR, 2, &format!("{name}: the back of the head"));
    }
}

#[test]
fn a_ghost_lets_the_ground_show_through() {
    let (x, y) = lone_tree();
    let ground = Vec2::new(x as f64 + 2.0, y as f64 + 1.0);
    let placed = |set: &SpriteSet| vec![Placed { sprite: set.id("player_down_idle").expect("there"), ground, flip: false, tint: [0.5; 4] }];
    let Some(shot) = shoot_scene("ghost", ground + Vec2::new(0.0, -0.5), 1.0, true, placed) else { return };
    // Half tunic, half grass, blended in the art's own numbers.
    let (tunic, grass) = (bytes_of_hex(TUNIC), bytes_of_hex(STUDIO_GRASS));
    let half = [0, 1, 2].map(|c| (tunic[c] as u32 + grass[c] as u32) / 2);
    close(shot.at(ground + Vec2::new(0.0, -0.45)), half[0] << 16 | half[1] << 8 | half[2], 3, "the ghost's tunic over grass");
}

const COPPER: u32 = 0xc78246;
const STONE: u32 = 0x8a8685;

#[test]
fn a_whole_rock_shows_its_copper_and_a_spent_one_is_bare_stone() {
    let world = sim::testmap::build();
    let (w, h) = (sim::testmap::WIDTH, sim::testmap::HEIGHT);
    // A rock with no tree below it tall enough to be drawn over its
    // copper (nothing else in the next rows reaches that high).
    let clear = |p: &sim::Prop| world.props_in(p.x - 1, p.y + 1, p.x + 2, p.y + 3).all(|below| below.kind != PropKind::Tree);
    let rock = world.props_in(4, 4, w - 4, h - 4).filter(|p| p.kind == PropKind::Rock).find(clear).expect("a rock with no tree in front of it");
    let ground = Vec2::new(rock.x as f64 + 0.5, rock.y as f64 + 0.5);
    // The middle of its biggest fleck of copper, from its anchor, in tiles.
    let fleck = ground + Vec2::new(-0.21, -0.29);
    let Some(whole) = shoot("rock-whole", ground, 1.0) else { return };
    close(whole.at(fleck), COPPER, 3, "a whole rock's copper");
    let Some(spent) = shoot_all("rock-spent", ground, 1.0, true, |_| Vec::new(), &|x, y| (x, y) == (rock.x, rock.y)) else { return };
    close(spent.at(fleck), STONE, 3, "a spent rock, where the copper was");
}
