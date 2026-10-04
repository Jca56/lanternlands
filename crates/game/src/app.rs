//! The game as the shell sees it: one area with no header, filled by
//! the world, which our own pass draws under the interface.

use lntrn_app::{AppHost, RenderCx};
use lntrn_core::{log_error, log_info};
use lntrn_math::{Rect, Vec2};
use lntrn_render::{Gpu, Images, TexDesc, wgpu};
use lntrn_ui::keymap::CTX_WINDOW;
use lntrn_ui::{Action, AreaCx, Host, HostCx, Key, KeyConfig, KeyItem, KeyPress, Modifiers, Sense, ShellRequest, Trigger, Ui, actions};
use sim::World;

use crate::assets::{self, SpriteSet};
use crate::camera::Camera;
use crate::render::WorldRenderer;
use crate::scene::{Look, Scene};

const FULLSCREEN: &str = "game.fullscreen";

/// How much one notch of the wheel zooms.
const ZOOM_PER_NOTCH: f64 = 1.15;
/// How fast the keys move the view, in logical pixels a second.
const KEY_PAN_SPEED: f64 = 1000.0;
/// The longest step the keys take at once, however long the last frame
/// took.
const LONGEST_STEP: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Editor {
    World,
}

pub struct Game {
    world: World,
    camera: Camera,
    sprites: SpriteSet,
    look: Look,
    scene: Scene,
    /// Made once the GPU exists.
    renderer: Option<WorldRenderer>,
    /// The world's area of the window in physical pixels, and the
    /// display's scale, as the last rebuild found them.
    view: Rect,
    scale: f64,
    /// The middle button is dragging the view.
    dragging: bool,
    /// When the keys last moved the view; `None` while none is held.
    key_step: Option<f64>,
    keys: KeyConfig,
    fullscreen: bool,
}

impl Game {
    pub fn new() -> Game {
        let dir = assets::assets_dir();
        let sprites = match SpriteSet::load(&dir) {
            Ok(set) => {
                log_info!("loaded {} sprites from {}", set.len(), dir.display());
                set
            }
            Err(e) => {
                log_error!("no sprites: {e}");
                SpriteSet::empty()
            }
        };
        let look = Look::find(&sprites);
        for name in look.missing() {
            log_error!("no sprite named {name}: what uses it is not drawn");
        }
        let mut keys = KeyConfig::default();
        keys.bind(CTX_WINDOW, KeyItem::new(Trigger::key(Key::Char('q'), Modifiers::CTRL), actions::QUIT));
        keys.bind(CTX_WINDOW, KeyItem::new(Trigger::key(Key::F(11), Modifiers::NONE), FULLSCREEN));
        Game {
            world: sim::testmap::build(),
            // The clearing in the middle of the test map.
            camera: Camera::new(Vec2::new(24.0, 14.0)),
            look,
            sprites,
            scene: Scene::default(),
            renderer: None,
            view: Rect::ZERO,
            scale: 1.0,
            dragging: false,
            key_step: None,
            keys,
            fullscreen: false,
        }
    }

    /// Which way the held keys push the view: arrows or WASD.
    fn key_push(held: &[Key]) -> Vec2 {
        let is = |key: &Key, arrow: Key, letter: char| *key == arrow || matches!(key, Key::Char(c) if c.eq_ignore_ascii_case(&letter));
        let mut push = Vec2::ZERO;
        for key in held {
            if is(key, Key::ArrowLeft, 'a') {
                push.x -= 1.0;
            } else if is(key, Key::ArrowRight, 'd') {
                push.x += 1.0;
            } else if is(key, Key::ArrowUp, 'w') {
                push.y -= 1.0;
            } else if is(key, Key::ArrowDown, 's') {
                push.y += 1.0;
            }
        }
        push
    }

    /// Move the view by what the pointer, the wheel and the keys did.
    fn steer(&mut self, ui: &mut Ui) {
        let over = ui.interact(ui.id("world"), self.view, Sense::CLICK).hovered;

        let notches = ui.state.wheel.y / ui.m.widget_h;
        if over && notches != 0.0 {
            self.camera.zoom_about(ZOOM_PER_NOTCH.powf(notches), ui.state.pointer, self.view, self.scale);
            ui.state.wheel = Vec2::ZERO;
        }

        self.dragging = ui.state.middle_down && (self.dragging || (over && ui.state.middle_pressed));
        if self.dragging {
            self.camera.pan_px(ui.state.delta, self.scale);
        }

        // With Ctrl or Alt held the keys mean something else.
        let free = !ui.state.mods.ctrl() && !ui.state.mods.alt();
        let push = if free { Self::key_push(&ui.state.keys_down) } else { Vec2::ZERO };
        if push == Vec2::ZERO {
            self.key_step = None;
        } else {
            let now = ui.now();
            let dt = self.key_step.map_or(0.0, |last| (now - last).clamp(0.0, LONGEST_STEP));
            self.camera.pan_px(-push * KEY_PAN_SPEED * self.scale * dt, self.scale);
            self.key_step = Some(now);
            ui.state.request_redraw_after(0.0);
        }

        if let Some([x0, y0, x1, y1]) = self.world.bounds() {
            self.camera.clamp_to(Rect::new(Vec2::new(x0 as f64, y0 as f64), Vec2::new(x1 as f64, y1 as f64)));
        }
    }
}

impl Host for Game {
    type Editor = Editor;
    type AreaState = ();

    fn editors(&self) -> &[Editor] {
        &[Editor::World]
    }

    fn editor_label(&self, _: Editor) -> &str {
        "World"
    }

    fn title(&self) -> String {
        "Explore the World".to_owned()
    }

    fn paints_body(&self, _: Editor) -> bool {
        true
    }

    fn shows_header(&self, _: Editor) -> bool {
        false
    }

    fn draw_body(&mut self, _: Editor, ui: &mut Ui, _: &mut AreaCx<()>) -> bool {
        self.view = ui.clip();
        self.scale = ui.m.scale;
        self.steer(ui);
        false
    }

    fn run(&mut self, action: &Action, cx: &mut HostCx) {
        if action.id == FULLSCREEN {
            self.fullscreen = !self.fullscreen;
            cx.request(ShellRequest::Fullscreen(self.fullscreen));
        }
    }

    fn key(&self, press: KeyPress, _: Option<Editor>) -> Option<Action> {
        self.keys.resolve(&[CTX_WINDOW], &press.to_event(), |_| true).map(KeyItem::action)
    }

    fn key_hint(&self, action: &Action) -> Option<String> {
        self.keys.hint_for(action)
    }
}

impl AppHost for Game {
    fn init_gpu(&mut self, gpu: &Gpu, format: wgpu::TextureFormat, _: &mut Images) {
        self.renderer = Some(WorldRenderer::new(gpu, format, &self.sprites.sheet));
    }

    fn render<'f>(&'f mut self, cx: &mut RenderCx<'f, '_>) {
        let Some(renderer) = self.renderer.as_mut().filter(|_| cx.window == 0) else { return };
        // The world's area, in whole pixels inside the window.
        let window = Rect::new(Vec2::ZERO, Vec2::new(cx.size[0] as f64, cx.size[1] as f64));
        let view = self.view.round().intersection(&window);
        if view.is_empty() {
            return;
        }
        let rect = [view.min.x as u32, view.min.y as u32, view.width() as u32, view.height() as u32];
        self.scene.build(&self.world, &self.camera, view.size(), self.scale, &self.sprites, &self.look);
        renderer.prepare(cx.gpu, &self.scene, [rect[2], rect[3]]);

        // Nothing more changes this frame: the passes only read.
        let renderer: &'f WorldRenderer = renderer;
        let picture = cx.graph.transient(TexDesc::color("world", rect[2], rect[3], WorldRenderer::TARGET_FORMAT));
        let backbuffer = cx.backbuffer;
        cx.graph.add_node("world", &[], &[picture], move |_, encoder, views| renderer.draw(encoder, views.get(picture)));
        cx.graph.add_node("world to window", &[picture], &[backbuffer], move |gpu, encoder, views| renderer.composite(gpu, encoder, views.get(picture), views.get(backbuffer), rect));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_and_wasd_push_the_same_way_and_opposites_cancel() {
        assert_eq!(Game::key_push(&[Key::ArrowLeft]), Vec2::new(-1.0, 0.0));
        assert_eq!(Game::key_push(&[Key::Char('a')]), Vec2::new(-1.0, 0.0));
        assert_eq!(Game::key_push(&[Key::Char('W'), Key::ArrowRight]), Vec2::new(1.0, -1.0));
        assert_eq!(Game::key_push(&[Key::Char('s'), Key::ArrowUp]), Vec2::ZERO);
        assert_eq!(Game::key_push(&[Key::Char('x'), Key::Space]), Vec2::ZERO);
    }
}
