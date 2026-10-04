//! The game as the shell sees it: the world, filling its area and drawn
//! by our own pass under the interface, and the map editor's panel
//! docked beside it while editing.

use std::path::{Path, PathBuf};

use lntrn_app::{AppHost, RenderCx};
use lntrn_core::{log_error, log_info};
use lntrn_math::{Color, Rect, Vec2};
use lntrn_render::{Gpu, Images, TexDesc, wgpu};
use lntrn_ui::keymap::CTX_WINDOW;
use lntrn_ui::{Action, AreaCx, Axis, CursorIcon, Dialog, Host, HostCx, Key, KeyConfig, KeyItem, KeyPress, Modifiers, Sense, Shell, ShellRequest, Trigger, Ui};
use sim::{PropKind, World};

use crate::assets::{self, SpriteSet};
use crate::camera::Camera;
use crate::editor::{MapEditor, Tool};
use crate::panel::{self, Asked, Pictures};
use crate::render::WorldRenderer;
use crate::scene::{Look, Scene};

const FULLSCREEN: &str = "game.fullscreen";
const QUIT: &str = "game.quit";
const QUIT_SAVED: &str = "game.quit_saved";
const QUIT_UNSAVED: &str = "game.quit_unsaved";
const EDIT: &str = "editor.toggle";
const UNDO: &str = "editor.undo";
const REDO: &str = "editor.redo";
const SAVE: &str = "editor.save";

/// The map the game plays and the editor edits, inside the assets.
const MAP: &str = "maps/overworld";

/// How much one notch of the wheel zooms.
const ZOOM_PER_NOTCH: f64 = 1.15;
/// How fast the keys move the view, in logical pixels a second.
const KEY_PAN_SPEED: f64 = 1000.0;
/// The longest step the keys take at once, however long the last frame
/// took.
const LONGEST_STEP: f64 = 0.1;
/// How wide the editor's panel opens, in logical pixels.
const PANEL_WIDTH: f64 = 400.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    World,
    Editor,
}

pub struct Game {
    world: World,
    /// Where the map is saved.
    map: PathBuf,
    /// Why the map did not load, until it has been said.
    load_error: Option<String>,
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
    editor: MapEditor,
    pictures: Pictures,
    /// The editor's panel is on screen.
    editing: bool,
    /// Open or close the panel after this rebuild.
    panel_wanted: Option<bool>,
}

impl Game {
    pub fn new() -> Game {
        let assets = assets::assets_dir();
        let map = assets.join(MAP);
        Game::open(&assets, map)
    }

    /// The game with its sprites from `dir` and its map in `map`.
    pub fn open(dir: &Path, map: PathBuf) -> Game {
        let sprites = match SpriteSet::load(dir) {
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
        let (world, load_error) = match World::load(&map) {
            Ok(world) => {
                log_info!("loaded {} chunks from {}", world.chunk_count(), map.display());
                (world, None)
            }
            Err(e) => {
                log_error!("the map did not load: {e}");
                (World::new(), Some(e))
            }
        };
        let middle = world.bounds().map_or(Vec2::ZERO, |[x0, y0, x1, y1]| Vec2::new((x0 + x1) as f64, (y0 + y1) as f64) * 0.5);

        let mut keys = KeyConfig::default();
        let mut bind = |key, mods, action: &str| keys.bind(CTX_WINDOW, KeyItem::new(Trigger::key(key, mods), action));
        bind(Key::Char('q'), Modifiers::CTRL, QUIT);
        bind(Key::F(11), Modifiers::NONE, FULLSCREEN);
        bind(Key::F(2), Modifiers::NONE, EDIT);
        bind(Key::Char('z'), Modifiers::CTRL, UNDO);
        bind(Key::Char('z'), Modifiers::CTRL | Modifiers::SHIFT, REDO);
        bind(Key::Char('y'), Modifiers::CTRL, REDO);
        bind(Key::Char('s'), Modifiers::CTRL, SAVE);
        Game {
            world,
            map,
            load_error,
            camera: Camera::new(middle),
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
            editor: MapEditor::new(),
            pictures: Pictures::default(),
            editing: false,
            panel_wanted: None,
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

    /// Move the view by what the wheel, the middle button and the keys
    /// did. `over` is whether the pointer is on the world.
    fn steer(&mut self, ui: &mut Ui, over: bool) {
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

        // The view may leave the map by a screen or so, to paint new
        // ground past its edge, but not wander off for good.
        if let Some([x0, y0, x1, y1]) = self.world.bounds() {
            let slack = self.camera.visible(self.view.size(), self.scale).size() * 0.5;
            self.camera.clamp_to(Rect::new(Vec2::new(x0 as f64, y0 as f64) - slack, Vec2::new(x1 as f64, y1 as f64) + slack));
        }
    }

    /// The world's area: steer the view and, while editing, work the
    /// held tool with the left button and show where it would land.
    fn draw_world(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>) {
        self.view = ui.clip();
        self.scale = ui.m.scale;
        if let Some(why) = self.load_error.take() {
            cx.request(ShellRequest::Dialog(Dialog::notice("The map did not load", &why)));
        }
        let hand = ui.interact(ui.id("world"), self.view, Sense::DRAG);
        self.steer(ui, hand.hovered);
        if !self.editing {
            self.editor.release();
            return;
        }

        let at = self.camera.to_world(ui.state.pointer, self.view, self.scale);
        if hand.pressed {
            self.editor.press(&mut self.world, at);
        } else if hand.dragging {
            self.editor.drag(&mut self.world, at);
        }
        if !hand.held {
            self.editor.release();
        }
        if hand.hovered || hand.held {
            if self.editor.tool == Tool::Move {
                // A hand over what can be picked up, a closed one carrying it.
                if self.editor.carrying().is_some() {
                    ui.state.cursor_icon = CursorIcon::Grabbing;
                } else if self.world.prop_at(at.x.floor() as i32, at.y.floor() as i32).is_some() {
                    ui.state.cursor_icon = CursorIcon::Pointer;
                }
            }
            // A carried prop is lit where it stands: under the pointer,
            // unless it cannot stand there.
            let lit = self.editor.carrying().map_or_else(|| self.editor.footprint(at), |tile| vec![tile]);
            let (fill, line) = (Color::rgba(1.0, 1.0, 1.0, 0.25), Color::rgba(1.0, 1.0, 1.0, 0.7));
            for (x, y) in lit {
                let corner = |x: i32, y: i32| self.camera.to_screen(Vec2::new(x as f64, y as f64), self.view, self.scale);
                let tile = Rect::new(corner(x, y), corner(x + 1, y + 1)).round();
                ui.draw.rect(tile, fill);
                ui.draw.stroke_rect(tile, ui.m.border, 0.0, line);
            }
        }
    }

    fn save(&mut self, cx: &mut HostCx) -> bool {
        self.editor.release();
        match self.world.save(&self.map) {
            Ok(_) => {
                cx.toast("Map saved");
                true
            }
            Err(why) => {
                log_error!("the map did not save: {why}");
                cx.request(ShellRequest::Dialog(Dialog::notice("The map did not save", &why)));
                false
            }
        }
    }

    /// Open or close the editor's panel if that was asked for, and note
    /// whether it is on screen. Returns whether the layout changed, so
    /// the frame is laid out again.
    fn sync_panel(&mut self, shell: &mut Shell<Self>) -> bool {
        let panel = shell.screen.target(View::Editor);
        let changed = match (self.panel_wanted.take(), panel, shell.screen.target(View::World)) {
            (Some(true), None, Some(world)) => {
                // The world keeps the first share of the split, so split
                // off the panel's width and then trade places.
                let share = PANEL_WIDTH * self.scale / self.view.width().max(1.0);
                shell.screen.split(world, Axis::Horizontal, share, View::Editor).is_some_and(|new| shell.screen.swap(world, new))
            }
            (Some(false), Some(panel), _) => shell.screen.join(panel),
            _ => false,
        };
        self.editing = shell.screen.target(View::Editor).is_some();
        changed
    }

    /// Ask what to do about unsaved changes before the game closes.
    fn ask_before_quitting(&self, cx: &mut HostCx) {
        let dialog = Dialog::new("Unsaved map changes", "The map has changes that are not saved.").button("Cancel", None).button("Discard", Some(Action::new(QUIT_UNSAVED))).button("Save", Some(Action::new(QUIT_SAVED))).default_button(2);
        cx.request(ShellRequest::Dialog(dialog));
    }
}

impl Host for Game {
    type Editor = View;
    type AreaState = ();

    fn editors(&self) -> &[View] {
        &[View::World, View::Editor]
    }

    fn editor_label(&self, view: View) -> &str {
        match view {
            View::World => "World",
            View::Editor => "Editor",
        }
    }

    fn title(&self) -> String {
        "Explore the World".to_owned()
    }

    fn paints_body(&self, view: View) -> bool {
        view == View::World
    }

    fn shows_header(&self, _: View) -> bool {
        false
    }

    fn draw_body(&mut self, view: View, ui: &mut Ui, cx: &mut AreaCx<()>) -> bool {
        match view {
            View::World => self.draw_world(ui, cx),
            View::Editor => match panel::draw(ui, &mut self.editor, self.world.is_dirty(), &self.pictures) {
                Some(Asked::Undo) => {
                    self.editor.undo(&mut self.world);
                }
                Some(Asked::Redo) => {
                    self.editor.redo(&mut self.world);
                }
                Some(Asked::Save) => {
                    self.save(&mut cx.host());
                }
                None => {}
            },
        }
        false
    }

    fn run(&mut self, action: &Action, cx: &mut HostCx) {
        match action.id.as_str() {
            FULLSCREEN => {
                self.fullscreen = !self.fullscreen;
                cx.request(ShellRequest::Fullscreen(self.fullscreen));
            }
            EDIT => self.panel_wanted = Some(!self.editing),
            UNDO if self.editing => {
                self.editor.undo(&mut self.world);
            }
            REDO if self.editing => {
                self.editor.redo(&mut self.world);
            }
            SAVE if self.editing => {
                self.save(cx);
            }
            QUIT if self.world.is_dirty() => self.ask_before_quitting(cx),
            QUIT | QUIT_UNSAVED => cx.quit(),
            QUIT_SAVED => {
                let saved = self.save(cx);
                if saved {
                    cx.quit();
                }
            }
            _ => {}
        }
        cx.rebuild();
    }

    fn key(&self, press: KeyPress, _: Option<View>) -> Option<Action> {
        self.keys.resolve(&[CTX_WINDOW], &press.to_event(), |_| true).map(KeyItem::action)
    }

    fn key_hint(&self, action: &Action) -> Option<String> {
        self.keys.hint_for(action)
    }

    fn close_requested(&mut self, _: bool, cx: &mut HostCx) -> bool {
        if self.world.is_dirty() {
            self.ask_before_quitting(cx);
        }
        !self.world.is_dirty()
    }
}

impl AppHost for Game {
    fn init_gpu(&mut self, gpu: &Gpu, format: wgpu::TextureFormat, images: &mut Images) {
        self.renderer = Some(WorldRenderer::new(gpu, format, &self.sprites.sheet));
        for (slot, kind) in self.pictures.props.iter_mut().zip(PropKind::ALL) {
            *slot = self.look.prop(kind).map(|id| images.add(gpu, self.sprites.picture(id)));
        }
    }

    fn after_rebuild(&mut self, _: &Gpu, _: &mut Images, shell: &mut Shell<Self>) -> bool {
        self.sync_panel(shell)
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
#[path = "app_tests.rs"]
mod tests;
