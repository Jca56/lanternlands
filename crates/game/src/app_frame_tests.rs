//! Whole frames drawn with no window: the world pass and the interface
//! over it, exactly as `lntrn-app` puts them together, read back as a
//! picture. Set `EXPLORE_RENDER_OUT` to a folder to keep each as a PNG.

use lntrn_image::Image;
use lntrn_render::{AtlasTexture, Pass2d, Readback, RenderGraph, TexturePool, clear_pass};

use super::play::{at_a_rock, pass, playing, press_tile};
use super::*;

/// Draw the frame the game would show now. `None`, and a line saying
/// so, on a machine with no GPU.
fn snap(name: &str, h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game) -> Option<Image> {
    let Some((gpu, _turn)) = crate::render::tests::gpu() else {
        eprintln!("SKIPPED {name}: no GPU to draw with");
        return None;
    };
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut images = Images::new(gpu);
    game.init_gpu(gpu, format, &mut images);
    // Rebuild once more so the draw list names the pictures just uploaded.
    let out = turn(h, shell, game);

    let size = [WINDOW.x as u32, WINDOW.y as u32];
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test window"),
        size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let mut pass2d = Pass2d::new(gpu, format, &images);
    let mut atlas = AtlasTexture::new(gpu, h.text.atlas());
    let mut pool = TexturePool::new();
    let mut encoder = gpu.create_encoder("test frame");
    {
        let (draw, text, images) = (&h.draw, &mut h.text, &images);
        let (pass2d, atlas) = (&mut pass2d, &mut atlas);
        let mut graph = RenderGraph::new();
        let backbuffer = graph.import(&view);
        graph.add_node("clear", &[], &[backbuffer], move |_, encoder, views| clear_pass(encoder, views.get(backbuffer), None, out.clear));
        game.render(&mut RenderCx { gpu, graph: &mut graph, backbuffer, depth: None, size, window: 0 });
        graph.add_node("ui", &[], &[backbuffer], move |gpu, encoder, views| pass2d.draw(gpu, encoder, views.get(backbuffer), size, draw, atlas, text.atlas_mut(), images, None));
        graph.execute(gpu, &mut pool, &mut encoder);
    }
    let read = Readback::begin(gpu, &mut encoder, &texture, true).expect("readable");
    gpu.queue.submit([encoder.finish()]);
    let image = read.finish(gpu).expect("the pixels come back");
    if let Some(dir) = std::env::var_os("EXPLORE_RENDER_OUT") {
        std::fs::write(Path::new(&dir).join(format!("{name}.png")), lntrn_image::encode_png(&image)).expect("the picture is written");
    }
    Some(image)
}

fn rgb(image: &Image, at: Vec2) -> [u8; 3] {
    let [r, g, b, _] = image.pixel(at.x as u32, at.y as u32);
    [r, g, b]
}

fn near(got: [u8; 3], want: [u8; 3], tolerance: u8) -> bool {
    got.iter().zip(&want).all(|(g, w)| g.abs_diff(*w) <= tolerance)
}

const GRASS: [u8; 3] = [0x73, 0xa1, 0x4c];
const COPPER: [u8; 3] = [0xc7, 0x82, 0x46];

/// How many pixels inside `rect` are within `tolerance` of a colour.
fn count(image: &Image, rect: Rect, colour: [u8; 3], tolerance: u8) -> usize {
    let (x0, y0, x1, y1) = (rect.min.x.ceil() as u32, rect.min.y.ceil() as u32, rect.max.x.floor() as u32, rect.max.y.floor() as u32);
    (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| near(rgb(image, Vec2::new(x as f64, y as f64)), colour, tolerance)).count()
}

fn body(shell: &Shell<Game>, view: View) -> WidgetId {
    WidgetId::ROOT.with_u64(shell.screen.target(view).expect("on screen") as u64).with("body")
}

#[test]
fn the_world_fills_the_window_and_the_buttons_sit_over_it() {
    let (mut h, mut shell, mut game) = playing("frame_plain");
    let Some(image) = snap("frame-play", &mut h, &mut shell, &mut game) else { return };
    // Grass in the far corner of the window: the world goes edge to edge.
    assert!(count(&image, Rect::new(Vec2::ZERO, WINDOW), GRASS, 1) > (WINDOW.x * WINDOW.y * 0.4) as usize, "the window is mostly grass");
    // Where the Bag button is, it is not grass.
    let bag = h.rect_of(body(&shell, View::World).with("Bag")).expect("a Bag button");
    assert_eq!(count(&image, bag.shrink(4.0), GRASS, 1), 0, "the button covers the grass under it");
}

#[test]
fn ore_in_the_bag_shows_its_picture_and_the_panels_cover_the_world() {
    let (mut h, mut shell, mut game, rock) = at_a_rock("frame_bag");
    press_tile(&mut h, &mut shell, &mut game, rock);
    for _ in 0..600 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        if game.sim.inventory.count(Item::CopperOre) > 0 {
            break;
        }
    }
    assert!(game.sim.inventory.count(Item::CopperOre) > 0);
    pass(&mut h, &mut shell, &mut game, 3.0);
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('k'), Modifiers::NONE);
    let Some(image) = snap("frame-bag-skills", &mut h, &mut shell, &mut game) else { return };

    let slot = |i: usize| h.rect_of(body(&shell, View::World).with("slot").with_index(i)).expect("a slot");
    // The first slot holds the ore: its copper shows. The next holds
    // nothing, and no slot shows the grass behind the panel.
    assert!(count(&image, slot(0), COPPER, 12) > 40, "the ore's picture is in the first slot");
    assert_eq!(count(&image, slot(1), COPPER, 12), 0);
    for i in [0, 1, 29] {
        assert_eq!(count(&image, slot(i), GRASS, 1), 0, "slot {i} lets the world show through");
    }
    // The world is still there beside the panels.
    assert!(near(rgb(&image, Vec2::new(60.0, 60.0)), GRASS, 1) || count(&image, Rect::new(Vec2::ZERO, Vec2::new(300.0, 300.0)), GRASS, 1) > 20_000);
}

#[test]
fn the_editors_panel_is_drawn_beside_the_world() {
    let (mut h, mut shell, mut game) = playing("frame_editor");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    let Some(image) = snap("frame-editor", &mut h, &mut shell, &mut game) else { return };
    let panel = shell.screen.layout_of(shell.screen.target(View::Editor).unwrap()).unwrap().rect;
    let world = shell.screen.layout_of(shell.screen.target(View::World).unwrap()).unwrap().rect;
    assert!(count(&image, world, GRASS, 1) > (world.area() * 0.4) as usize, "the world beside it");
    // The only grass colour in the panel is the Grass row's swatch.
    let grass_row = h.rect_of(body(&shell, View::Editor).with("map editor").with("Grass")).expect("a Grass row");
    let swatch = count(&image, grass_row, GRASS, 1);
    assert!(swatch > 500, "the swatch is drawn");
    assert_eq!(count(&image, panel, GRASS, 1), swatch, "no world under the panel");
}

#[test]
fn a_level_gained_is_said_at_the_top_and_leaves_the_buttons_clear() {
    let (mut h, mut shell, mut game, rock) = at_a_rock("frame_level");
    press_tile(&mut h, &mut shell, &mut game, rock);
    for _ in 0..600 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        if !game.hud.said().is_empty() {
            break;
        }
    }
    assert_eq!(game.hud.said(), vec!["Mining level 2"]);
    let Some(image) = snap("frame-level-up", &mut h, &mut shell, &mut game) else { return };
    // White letters on a dark plate in the top middle of the window.
    let top = Rect::new(Vec2::new(WINDOW.x * 0.3, 0.0), Vec2::new(WINDOW.x * 0.7, 120.0));
    assert!(count(&image, top, [255, 255, 255], 40) > 300, "the message's letters");
    // And the buttons are as they are with nothing said: nothing over them.
    let bag = h.rect_of(body(&shell, View::World).with("Bag")).expect("a Bag button");
    pass(&mut h, &mut shell, &mut game, 6.0);
    assert!(game.hud.said().is_empty(), "the message goes away by itself");
    let Some(after) = snap("frame-level-up-after", &mut h, &mut shell, &mut game) else { return };
    let same = |a: &Image, b: &Image, rect: Rect| (rect.min.y as u32..rect.max.y as u32).all(|y| (rect.min.x as u32..rect.max.x as u32).all(|x| a.pixel(x, y) == b.pixel(x, y)));
    assert!(same(&image, &after, bag.round()), "the Bag button looked the same while the message showed");
}
