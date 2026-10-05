//! The game as the shell sees it: the world, filling its area and drawn
//! by our own pass under the interface, and the map editor's panel
//! docked beside it while editing.

use std::path::{Path, PathBuf};

use lntrn_app::{AppHost, Exit, RenderCx};
use lntrn_core::{log_error, log_info};
use lntrn_math::{Rect, Vec2};
use lntrn_render::{Gpu, Images, TexDesc, wgpu};
use lntrn_ui::keymap::CTX_WINDOW;
use lntrn_ui::{Action, AreaCx, Axis, Dialog, Host, HostCx, Key, KeyConfig, KeyItem, KeyPress, Modifiers, Shell, ShellRequest, Trigger, Ui};
use lntrn_image::Image;
use lntrn_ui::ImageHandle;
use sim::{Ability, Character, Item, MonsterKind, PersonKind, PropKind, Sim, TICK_SECONDS, World};

use crate::anim::{Facing, Pose};
use crate::assets::{self, SpriteSet};
use crate::camera::Camera;
use crate::editor::MapEditor;
use crate::hotbar::{self, AbilityPictures};
use crate::hud::{Hud, ItemPictures};
use crate::panel::{self, Asked, Pictures};
use crate::render::WorldRenderer;
use crate::saves::{SLOTS, Saves};
use crate::title::Slot;
use crate::scene::{Look, Scene};

const FULLSCREEN: &str = "game.fullscreen";
const QUIT: &str = "game.quit";
const QUIT_SAVED: &str = "game.quit_saved";
const QUIT_UNSAVED: &str = "game.quit_unsaved";
const EDIT: &str = "editor.toggle";
const UNDO: &str = "editor.undo";
const REDO: &str = "editor.redo";
const SAVE: &str = "editor.save";
const BAG: &str = "hud.bag";
const SKILLS: &str = "hud.skills";
const GEAR: &str = "hud.gear";
/// Hear the next thing whoever is talking has to say.
const TALK: &str = "hud.talk";
/// The hotbar's buttons, left to right: the keys 1 to 6.
const SLOT_KEYS: [&str; hotbar::SLOTS] = ["hud.slot.1", "hud.slot.2", "hud.slot.3", "hud.slot.4", "hud.slot.5", "hud.slot.6"];
const TITLE: &str = "game.title";
const CREATE: &str = "title.create";
const DELETE: &str = "title.delete";

/// The game's name to the window system, and its folder under
/// Lantern's `save/`.
pub const APP_ID: &str = "lanternlands";
/// While playing, the game is saved at least this often, in seconds,
/// if anything has happened.
const AUTOSAVE_EVERY: f64 = 10.0;

/// The map the game plays and the editor edits, inside the assets.
const MAP: &str = "maps/overworld";
const TITLE_PAINTING: &str = "title/title-01.png";

/// How wide the editor's panel opens, in logical pixels.
const PANEL_WIDTH: f64 = 400.0;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    World,
    Editor,
}

/// What the window is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Title,
    /// The character in `slot` is being played.
    Playing { slot: usize },
}

pub struct Game {
    mode: Mode,
    saves: Saves,
    /// What each slot holds, as the title screen shows it.
    slots: [Slot; SLOTS],
    /// The character in play, and how long they have been played, in
    /// seconds.
    character: Character,
    played: f64,
    /// Something happened since the game was last saved, at `saved_at`.
    unsaved: bool,
    saved_at: f64,
    /// The slot a new character is being named for, and the name so far.
    naming: Option<(usize, String)>,
    /// The slot whose character is being asked about deleting.
    deleting: Option<usize>,
    /// The title painting: as loaded, and as the interface draws it.
    title_image: Option<Image>,
    title_picture: Option<ImageHandle>,
    /// Seed the dice from the clock when a character is entered.
    dice_from_clock: bool,
    sim: Sim,
    /// When the last frame was, to know how long this one took.
    clock: Option<f64>,
    /// Seconds into the tick under way. Parked at a whole tick while
    /// nothing is happening, so the first thing that does starts at once.
    phase: f64,
    /// Seconds into the world's tick under way.
    world_phase: f64,
    /// The way the player last stepped, and each monster and person.
    facing: Facing,
    monster_facing: Vec<Facing>,
    person_facing: Vec<Facing>,
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
    hud: Hud,
    /// The items' pictures: as loaded, and as the interface draws them.
    item_images: Vec<Option<Image>>,
    item_pictures: ItemPictures,
    ability_images: Vec<Option<Image>>,
    ability_pictures: AbilityPictures,
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
        let mut game = Game::open(&assets, map, Saves::default_dir(APP_ID));
        // Different dice every time a character is played.
        game.dice_from_clock = true;
        game
    }

    /// The game with its sprites from `dir`, its map in `map` and its
    /// characters in `saves`. It opens on the title screen.
    pub fn open(dir: &Path, map: PathBuf, saves: PathBuf) -> Game {
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
        let picture = |folder: &str, name: &str| {
            let path = dir.join(format!("{folder}/{name}.png"));
            let image = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| lntrn_image::decode(&bytes).map_err(|e| e.to_string()));
            image.map_err(|e| log_error!("no picture for {name}: {}: {e}", path.display())).ok()
        };
        let item_images = Item::ALL.map(|item| picture("items", item.name()));
        let ability_images = Ability::ALL.map(|ability| picture("abilities", ability.name()));
        let title_path = dir.join(TITLE_PAINTING);
        let title_image = std::fs::read(&title_path).map_err(|e| e.to_string()).and_then(|bytes| lntrn_image::decode(&bytes).map_err(|e| e.to_string()));
        let title_image = title_image.map_err(|e| log_error!("no title painting: {}: {e}", title_path.display())).ok();
        let sim = Sim::new(world);
        let (x, y) = sim.player.tile;

        let mut keys = KeyConfig::default();
        let mut bind = |key, mods, action: &str| keys.bind(CTX_WINDOW, KeyItem::new(Trigger::key(key, mods), action));
        bind(Key::Char('q'), Modifiers::CTRL, QUIT);
        bind(Key::F(11), Modifiers::NONE, FULLSCREEN);
        bind(Key::F(2), Modifiers::NONE, EDIT);
        bind(Key::Char('z'), Modifiers::CTRL, UNDO);
        bind(Key::Char('z'), Modifiers::CTRL | Modifiers::SHIFT, REDO);
        bind(Key::Char('y'), Modifiers::CTRL, REDO);
        bind(Key::Char('s'), Modifiers::CTRL, SAVE);
        bind(Key::Char('i'), Modifiers::NONE, BAG);
        bind(Key::Char('k'), Modifiers::NONE, SKILLS);
        bind(Key::Char('g'), Modifiers::NONE, GEAR);
        for (i, slot) in SLOT_KEYS.iter().enumerate() {
            bind(Key::Char((b'1' + i as u8) as char), Modifiers::NONE, slot);
        }
        bind(Key::Space, Modifiers::NONE, TALK);
        bind(Key::Enter, Modifiers::NONE, TALK);
        bind(Key::Escape, Modifiers::NONE, TITLE);
        let mut game = Game {
            mode: Mode::Title,
            saves: Saves::new(saves),
            slots: std::array::from_fn(|_| Slot::Empty),
            character: Character::default(),
            played: 0.0,
            unsaved: false,
            saved_at: 0.0,
            naming: None,
            deleting: None,
            title_image,
            title_picture: None,
            dice_from_clock: false,
            sim,
            clock: None,
            phase: TICK_SECONDS,
            world_phase: 0.0,
            facing: Facing::default(),
            monster_facing: Vec::new(),
            person_facing: Vec::new(),
            map,
            load_error,
            camera: Camera::new(Vec2::new(x as f64 + 0.5, y as f64 + 0.5)),
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
            hud: Hud::default(),
            item_images: item_images.to_vec(),
            item_pictures: ItemPictures::default(),
            ability_images: ability_images.to_vec(),
            ability_pictures: AbilityPictures::default(),
            editor: MapEditor::new(),
            pictures: Pictures::default(),
            editing: false,
            panel_wanted: None,
        };
        game.refresh_slots();
        game
    }

    /// Quit the game: at once, or after asking about map changes that
    /// are not saved. The character in play is saved either way.
    fn quit(&mut self, cx: &mut HostCx) {
        self.autosave();
        if self.sim.world.is_dirty() {
            self.ask_before_quitting(cx);
        } else {
            cx.quit();
        }
    }

    fn save(&mut self, cx: &mut HostCx) -> bool {
        self.editor.release();
        match self.sim.world.save(&self.map) {
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
        let was_editing = std::mem::replace(&mut self.editing, shell.screen.target(View::Editor).is_some());
        if was_editing && !self.editing {
            // Back to the game: on ground that may have changed under them.
            self.sim.settle_player();
            self.phase = TICK_SECONDS;
        }
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
        "Lanternlands".to_owned()
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
            View::Editor => match panel::draw(ui, &mut self.editor, self.sim.world.is_dirty(), &self.pictures) {
                Some(Asked::Undo) => {
                    self.editor.undo(&mut self.sim.world);
                }
                Some(Asked::Redo) => {
                    self.editor.redo(&mut self.sim.world);
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
        let playing = self.mode != Mode::Title;
        match action.id.as_str() {
            FULLSCREEN => {
                self.fullscreen = !self.fullscreen;
                cx.request(ShellRequest::Fullscreen(self.fullscreen));
            }
            EDIT if playing => self.panel_wanted = Some(!self.editing),
            BAG if playing && !self.editing => self.hud.bag_open = !self.hud.bag_open,
            SKILLS if playing && !self.editing => self.hud.skills_open = !self.hud.skills_open,
            GEAR if playing && !self.editing => self.hud.gear_open = !self.hud.gear_open,
            slot if playing && !self.editing && SLOT_KEYS.contains(&slot) => self.use_slot(SLOT_KEYS.iter().position(|key| *key == slot).unwrap_or(0)),
            TALK if playing && !self.editing => self.sim.talk_on(),
            // Esc closes what is open before it leaves the game.
            TITLE if playing && !self.editing && self.sim.saying().is_some() => self.sim.end_talk(),
            TITLE if playing && !self.editing && self.sim.station().is_some() => self.sim.leave_station(),
            TITLE if playing && !self.editing => self.back_to_title(),
            CREATE => self.create_character(),
            DELETE => self.delete_character(cx),
            UNDO if self.editing => {
                self.editor.undo(&mut self.sim.world);
            }
            REDO if self.editing => {
                self.editor.redo(&mut self.sim.world);
            }
            SAVE if self.editing => {
                self.save(cx);
            }
            QUIT => self.quit(cx),
            QUIT_UNSAVED => cx.quit(),
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

    fn draw_item(&mut self, key: &str, ui: &mut Ui, cx: &mut HostCx) -> bool {
        key == view_title::NAME_FIELD && self.draw_name_field(ui, cx)
    }

    fn dialog_ready(&self, action: &Action) -> bool {
        action.id != CREATE || self.name_ready()
    }

    fn close_requested(&mut self, _: bool, cx: &mut HostCx) -> bool {
        self.autosave();
        if self.sim.world.is_dirty() {
            self.ask_before_quitting(cx);
        }
        !self.sim.world.is_dirty()
    }
}

impl AppHost for Game {
    fn init_gpu(&mut self, gpu: &Gpu, format: wgpu::TextureFormat, images: &mut Images) {
        self.renderer = Some(WorldRenderer::new(gpu, format, &self.sprites.sheet));
        for (slot, kind) in self.pictures.props.iter_mut().zip(PropKind::ALL) {
            *slot = self.look.prop(kind).map(|id| images.add(gpu, self.sprites.picture(id)));
        }
        for (slot, image) in self.item_pictures.0.iter_mut().zip(&self.item_images) {
            *slot = image.as_ref().map(|image| images.add(gpu, image));
        }
        for (slot, image) in self.ability_pictures.0.iter_mut().zip(&self.ability_images) {
            *slot = image.as_ref().map(|image| images.add(gpu, image));
        }
        for (slot, kind) in self.pictures.monsters.iter_mut().zip(MonsterKind::ALL) {
            let standing = self.look.monster(kind).sprite(Facing::Down, Pose::Idle);
            *slot = standing.map(|(id, _)| images.add(gpu, self.sprites.picture(id)));
        }
        for (slot, kind) in self.pictures.people.iter_mut().zip(PersonKind::ALL) {
            let standing = self.look.person(kind).sprite(Facing::Down, Pose::Idle);
            *slot = standing.map(|(id, _)| images.add(gpu, self.sprites.picture(id)));
        }
        self.title_picture = self.title_image.as_ref().map(|image| images.add(gpu, image));
    }

    fn after_rebuild(&mut self, _: &Gpu, _: &mut Images, shell: &mut Shell<Self>) -> bool {
        self.sync_panel(shell)
    }

    fn exiting(&mut self, _: Exit) {
        // However the game ended, the character in play is kept.
        self.autosave();
    }

    fn render<'f>(&'f mut self, cx: &mut RenderCx<'f, '_>) {
        if self.mode == Mode::Title {
            // The painting covers the window; there is no world to draw.
            return;
        }
        let extras = self.extras();
        let Some(renderer) = self.renderer.as_mut().filter(|_| cx.window == 0) else { return };
        // The world's area, in whole pixels inside the window.
        let window = Rect::new(Vec2::ZERO, Vec2::new(cx.size[0] as f64, cx.size[1] as f64));
        let view = self.view.round().intersection(&window);
        if view.is_empty() {
            return;
        }
        let rect = [view.min.x as u32, view.min.y as u32, view.width() as u32, view.height() as u32];
        let sim = &self.sim;
        self.scene.build(&sim.world, &self.camera, view.size(), self.scale, &self.sprites, &self.look, &extras, &|x, y| sim.is_spent((x, y)));
        renderer.prepare(cx.gpu, &self.scene, [rect[2], rect[3]]);

        // Nothing more changes this frame: the passes only read.
        let renderer: &'f WorldRenderer = renderer;
        let picture = cx.graph.transient(TexDesc::color("world", rect[2], rect[3], WorldRenderer::TARGET_FORMAT));
        let backbuffer = cx.backbuffer;
        cx.graph.add_node("world", &[], &[picture], move |_, encoder, views| renderer.draw(encoder, views.get(picture)));
        cx.graph.add_node("world to window", &[picture], &[backbuffer], move |gpu, encoder, views| renderer.composite(gpu, encoder, views.get(picture), views.get(backbuffer), rect));
    }
}

#[path = "app_view.rs"]
mod view;

#[path = "app_title.rs"]
mod view_title;

#[path = "app_people.rs"]
mod view_people;

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
