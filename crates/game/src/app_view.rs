//! The world's area of the window: the game while playing, and the
//! held tool's workbench while the map editor is open.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{AreaCx, CursorIcon, Dialog, Key, Response, Sense, ShellRequest, Ui};
use sim::{Ability, Activity, Event, Marker, TICK_SECONDS};

use super::{AUTOSAVE_EVERY, Game, Mode};
use crate::anim::{Facing, Pose};
use crate::assets::SpriteId;
use crate::bank_panel::{self, Asked};
use crate::editor::Tool;
use crate::hotbar;
use crate::hud::{Press, Sight};
use crate::making;
use crate::scene::{GEAR_ORDER, Placed, metal_tint};
use crate::talk;
use crate::wear::dress;

/// How much one notch of the wheel zooms.
const ZOOM_PER_NOTCH: f64 = 1.15;
/// How fast the keys move the view, in logical pixels a second.
const KEY_PAN_SPEED: f64 = 1000.0;
/// The longest step anything smooth takes at once (the keys' push, the
/// view's glide), however long the last frame took.
const LONGEST_STEP: f64 = 0.1;
/// The longest the game's clock runs on from one frame to the next.
const LONGEST_WAIT: f64 = 1.0;
/// The part of somebody a click counts as on them, in tiles from their
/// feet: this far to either side, this far down, and as far up as they
/// are tall, which for a monster is this.
const AIM_HALF_WIDTH: f64 = 0.45;
const AIM_HEIGHT: f64 = 1.15;
const AIM_BELOW: f64 = 0.15;
/// How quickly the view closes on the player: the share of the gap it
/// covers each second is all but `e` to minus this.
const FOLLOW_RATE: f64 = 8.0;
/// Closer to the player than this, in tiles, the view is on them.
const FOLLOW_SETTLED: f64 = 0.002;
/// The ring on the tile the player is walking to.
const GOAL_RING: Color = Color::rgba(0.757, 0.608, 0.290, 0.9);
/// How see-through the editor's ghosts are: the player on their start,
/// and whoever lives on a tile.
pub(super) const GHOST: f32 = 0.5;
/// How far up the screen, in tiles, what lies on the ground under
/// someone is put, to be drawn before them. Far too little to see.
const UNDER: f64 = 1e-4;
/// The colours of the ring on the ground under what the player is set
/// on: a monster to fight, a thing to use (the walk ring's gold), a
/// person to talk with.
pub(super) const RING_FIGHT: [f32; 4] = [226.0 / 255.0, 72.0 / 255.0, 60.0 / 255.0, 1.0];
pub(super) const RING_USE: [f32; 4] = [0.757, 0.608, 0.290, 1.0];
pub(super) const RING_TALK: [f32; 4] = [0.471, 0.682, 0.941, 1.0];

/// A ring on the ground at `ground`, in one of the rings' colours: a
/// hair up the screen, so it is drawn before what stands there.
pub(super) fn ring(sprite: Option<SpriteId>, ground: Vec2, tint: [f32; 4]) -> Option<Placed> {
    sprite.map(|sprite| Placed { sprite, ground: ground - Vec2::new(0.0, UNDER), flip: false, tint })
}

/// Whether a point of the world, in tiles, is on somebody `height`
/// tiles tall whose feet are at `feet`.
pub(super) fn on_body(at: Vec2, feet: Vec2, height: f64) -> bool {
    (at.x - feet.x).abs() <= AIM_HALF_WIDTH && at.y >= feet.y - height && at.y <= feet.y + AIM_BELOW
}

impl Game {
    /// Which way the held keys push the view: arrows or WASD.
    pub(super) fn key_push(held: &[Key]) -> Vec2 {
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

    /// The wheel zooms about the pointer, playing or editing.
    fn zoom(&mut self, ui: &mut Ui, over: bool) {
        let notches = ui.state.wheel.y / ui.m.widget_h;
        if over && notches != 0.0 {
            self.camera.zoom_about(ZOOM_PER_NOTCH.powf(notches), ui.state.pointer, self.view, self.scale);
            ui.state.wheel = Vec2::ZERO;
        }
    }

    /// The editor's free view: the middle button drags it and the keys
    /// push it. `over` is whether the pointer is on the world.
    fn steer(&mut self, ui: &mut Ui, over: bool) {
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
        if let Some([x0, y0, x1, y1]) = self.sim.world.bounds() {
            let slack = self.camera.visible(self.view.size(), self.scale).size() * 0.5;
            self.camera.clamp_to(Rect::new(Vec2::new(x0 as f64, y0 as f64) - slack, Vec2::new(x1 as f64, y1 as f64) + slack));
        }
    }

    /// Where the player is drawn: sliding from the tile they left to the
    /// one they are on over the course of the tick.
    pub(super) fn player_ground(&self) -> Vec2 {
        let player = &self.sim.player;
        let middle = |(x, y): (i32, i32)| Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
        let into = (self.phase / TICK_SECONDS).clamp(0.0, 1.0);
        middle(player.from).lerp(middle(player.tile), into)
    }

    /// The world's area: the game while playing, the held tool's
    /// workbench while editing.
    pub(super) fn draw_world(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>) {
        self.view = ui.clip();
        self.scale = ui.m.scale;
        if let Some(why) = self.load_error.take() {
            cx.request(ShellRequest::Dialog(Dialog::notice("The map did not load", &why)));
        }
        let now = ui.now();
        // The game's clock keeps real time between frames, short of a
        // stall; a frame that long is not a second of the game.
        let dt = self.clock.replace(now).map_or(0.0, |last| (now - last).clamp(0.0, LONGEST_WAIT));
        if self.mode == Mode::Title {
            return self.draw_title(ui, cx);
        }
        let hand = ui.interact(ui.id("world"), self.view, Sense::DRAG);
        self.zoom(ui, hand.hovered);
        if self.editing {
            self.steer(ui, hand.hovered);
            self.edit(ui, &hand);
        } else {
            self.editor.release();
            self.play(ui, &hand, dt);
            if let Some(menu) = self.menu_wanted.take() {
                cx.request(ShellRequest::ContextMenu(Box::new(menu)));
            }
        }
    }

    /// Playing: a press sets the player on what is under the pointer (a
    /// monster to fight, a person to talk with, a rock to work, the
    /// grave to open) or walks them there; the world's clock and the player's tick; the view
    /// glides after the player; and the interface floats over it all.
    fn play(&mut self, ui: &mut Ui, hand: &Response, dt: f64) {
        let at = self.camera.to_world(ui.state.pointer, self.view, self.scale);
        let under = (at.x.floor() as i32, at.y.floor() as i32);
        let aimed = if hand.hovered { self.monster_under(ui.state.pointer) } else { None };
        let hailed = if hand.hovered && aimed.is_none() { self.person_under(ui.state.pointer) } else { None };
        let usable = self.sim.node_left(under).is_some_and(|left| left > 0) || self.sim.station_at(under).is_some() || self.sim.bank_at(under) || self.sim.grave.as_ref().is_some_and(|g| g.tile == under);
        if hand.hovered && (aimed.is_some() || hailed.is_some() || usable) {
            ui.state.cursor_icon = CursorIcon::Pointer;
        }
        if hand.pressed {
            if !self.sim.player.busy() {
                // From a standstill, the player's first tick comes at once.
                self.phase = TICK_SECONDS;
            }
            match (aimed, hailed) {
                (Some(monster), _) => self.sim.attack(monster),
                (None, Some(person)) => self.sim.talk_to(person),
                (None, None) => self.sim.click(under),
            }
        }
        if hand.hovered && ui.state.right_pressed {
            // A right press asks what can be done here instead.
            self.menu_wanted = Some(self.world_menu(ui.state.pointer));
        }
        // The world keeps its own time, steadily, while anything in it
        // has something to do.
        if self.sim.waiting() {
            self.world_phase += dt;
            while self.world_phase >= TICK_SECONDS {
                self.world_phase -= TICK_SECONDS;
                self.sim.tick_world();
                self.face_monsters();
            }
        } else {
            self.world_phase = 0.0;
        }
        // The player's runs only while they have something to do (or a
        // swing to finish), so it can start the moment they do.
        if self.sim.acting() {
            self.phase += dt;
            while self.phase >= TICK_SECONDS {
                self.phase -= TICK_SECONDS;
                self.sim.tick_player();
                self.face();
            }
        }
        if !self.sim.acting() {
            self.phase = TICK_SECONDS;
        }
        self.face_people();
        // Walking up to the bank opens the bag beside it.
        let banking = self.sim.banking();
        self.hud.bag_open |= banking && !self.was_banking;
        self.was_banking = banking;
        let now = ui.now();
        let events = self.sim.take_events();
        self.hud.hear(&events, &self.sim, now);
        if events.contains(&Event::Died) {
            // They woke somewhere else: the view is there at once.
            self.camera.center = self.player_ground();
        }
        // Time at the game is time played; what is earned or lost is
        // saved at once, and the rest every so often.
        self.played += dt;
        self.unsaved |= self.sim.player.busy();
        let kept = events.iter().any(|e| matches!(e, Event::Got { .. } | Event::LevelUp { .. } | Event::Died | Event::GraveOpened { .. }));
        if kept || (self.unsaved && now - self.saved_at >= AUTOSAVE_EVERY) {
            self.autosave();
        }

        let target = self.player_ground();
        let gap = target - self.camera.center;
        let following = gap.length() > FOLLOW_SETTLED;
        let smooth = dt.min(LONGEST_STEP);
        self.camera.center = if following { self.camera.center + gap * (1.0 - (-FOLLOW_RATE * smooth).exp()) } else { target };

        // Where they are walking to is ringed on the ground; what they
        // are going for (a monster, a rock, a person) is ringed itself,
        // in the world.
        let set_on = self.sim.target().is_some() || self.sim.using().is_some() || self.sim.awaited().is_some();
        if let Some((x, y)) = self.sim.player.goal().filter(|_| !set_on) {
            let middle = self.camera.to_screen(Vec2::new(x as f64 + 0.5, y as f64 + 0.5), self.view, self.scale);
            let tile_px = self.camera.tile_px(self.scale);
            ui.draw.ring(middle, tile_px * 0.3, (tile_px * 0.06).max(2.0), GOAL_RING);
        }
        let (camera, view, scale) = (self.camera, self.view, self.scale);
        let monsters = self.monster_grounds();
        talk::draw_names(ui, &self.sim, &self.person_grounds(), &|world| camera.to_screen(world, view, scale));
        let sight = Sight { player: target, to_screen: &|world| camera.to_screen(world, view, scale), monsters: &monsters, tile_px: camera.tile_px(scale), into: self.world_phase / TICK_SECONDS };
        let wear = self.hud.draw(ui, self.view, &self.sim, &self.item_pictures, &sight);
        let pointer = ui.state.pointer;
        match wear {
            // At the bank a press on the bag puts one in; anywhere else
            // it wears or drinks.
            Some(Press::Bag(slot)) if banking => self.put_one(slot),
            Some(Press::Bag(slot)) => {
                self.sim.use_slot(slot);
                // What is worn is kept at once, like what is earned.
                self.autosave();
            }
            Some(Press::Worn(slot)) => {
                self.sim.unequip(slot);
                self.autosave();
            }
            Some(Press::BagMenu(slot)) => self.menu_wanted = self.bag_menu(slot, pointer),
            Some(Press::WornMenu(slot)) => self.menu_wanted = self.worn_menu(slot, pointer),
            None => {}
        }
        let into = self.world_phase / TICK_SECONDS;
        if let Some(ability) = hotbar::draw(ui, self.view, &self.sim, &self.ability_pictures, into) {
            self.use_ability(ability);
        }
        if let Some((recipe, count)) = making::draw(ui, self.view, &self.sim, &self.item_pictures) {
            if !self.sim.player.busy() {
                self.phase = TICK_SECONDS;
            }
            self.sim.make(recipe, count);
        }
        let asked = bank_panel::draw(ui, self.view, &self.sim, &self.item_pictures);
        match asked {
            Some(Asked::Take(slot)) => self.take_one(slot),
            Some(Asked::Menu(slot)) => self.menu_wanted = self.bank_menu(slot, pointer),
            Some(Asked::PutBag) => {
                self.sim.deposit_all();
                self.autosave();
            }
            None => {}
        }
        if talk::draw(ui, self.view, &self.sim) {
            self.sim.talk_on();
            ui.state.request_redraw_after(0.0);
        }
        if wear.is_some() || asked.is_some() {
            // What a press changed shows in the next frame.
            ui.state.request_redraw_after(0.0);
        }

        // Every frame while something moves; else only when the world's
        // next tick is due, if one will change anything; else not at all.
        if self.sim.acting() || self.sim.cooling() || self.sim.stirring() || following || self.hud.animating(now) {
            ui.state.request_redraw_after(0.0);
        } else if self.sim.waiting() {
            ui.state.request_redraw_after(TICK_SECONDS - self.world_phase);
        }
    }

    /// Work a button of the hotbar, by its place on the bar.
    pub(super) fn use_slot(&mut self, slot: usize) {
        if let Some(ability) = hotbar::BAR.get(slot).copied().flatten() {
            self.use_ability(ability);
        }
    }

    /// Ask for an ability: used on the player's next tick, which from a
    /// standstill comes at once.
    fn use_ability(&mut self, ability: Ability) {
        if !self.sim.player.busy() {
            self.phase = TICK_SECONDS;
        }
        self.sim.use_ability(ability);
    }

    /// Turn the player the way they last stepped, or toward what they
    /// are working at or fighting, or whoever is talking to them.
    fn face(&mut self) {
        let player = &self.sim.player;
        let toward = |at: (i32, i32)| (at.0 - player.tile.0, at.1 - player.tile.1);
        let way = match player.activity() {
            Some(Activity::Working { at, .. } | Activity::Making { at, .. }) => toward(at),
            Some(Activity::Fighting { target, .. }) => self.sim.monsters.get(target).map_or((0, 0), |m| toward(m.tile)),
            None => match self.sim.saying().and_then(|saying| self.sim.people.get(saying.person)) {
                Some(person) => toward(person.tile),
                None => (player.tile.0 - player.from.0, player.tile.1 - player.from.1),
            },
        };
        self.facing = Facing::of_step(way.0, way.1).unwrap_or(self.facing);
    }

    /// Turn each monster the way it last stepped, or toward the player
    /// it just swung at.
    fn face_monsters(&mut self) {
        self.monster_facing.resize(self.sim.monsters.len(), Facing::default());
        let (player, now) = (self.sim.player.tile, self.sim.ticks);
        for (i, (monster, facing)) in self.sim.monsters.iter().zip(&mut self.monster_facing).enumerate() {
            let way = if monster.swung_at == Some(now) || self.sim.monster_swing_due(i) { (player.0 - monster.tile.0, player.1 - monster.tile.1) } else { (monster.tile.0 - monster.from.0, monster.tile.1 - monster.from.1) };
            *facing = Facing::of_step(way.0, way.1).unwrap_or(*facing);
        }
    }

    /// Where each monster's feet are drawn, in tiles: sliding from the
    /// tile it left to the one it is on over the world's tick.
    pub(super) fn monster_grounds(&self) -> Vec<Vec2> {
        let into = (self.world_phase / TICK_SECONDS).clamp(0.0, 1.0);
        let middle = |(x, y): (i32, i32)| Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
        self.sim.monsters.iter().map(|m| middle(m.from).lerp(middle(m.tile), into)).collect()
    }

    /// The living monster drawn under a point of the screen: the one
    /// nearest the front, where two overlap.
    pub(super) fn monster_under(&self, pointer: Vec2) -> Option<usize> {
        let at = self.camera.to_world(pointer, self.view, self.scale);
        let grounds = self.monster_grounds();
        let over = |feet: &Vec2| on_body(at, *feet, AIM_HEIGHT);
        let alive = |&(i, _): &(usize, &Vec2)| self.sim.monsters[i].alive();
        grounds.iter().enumerate().filter(alive).filter(|(_, feet)| over(feet)).max_by(|a, b| a.1.y.total_cmp(&b.1.y)).map(|(i, _)| i)
    }

    /// Editing: the left button works the held tool on the world, and
    /// the tiles it would touch are lit under the pointer.
    fn edit(&mut self, ui: &mut Ui, hand: &Response) {
        let at = self.camera.to_world(ui.state.pointer, self.view, self.scale);
        if hand.pressed {
            self.editor.press(&mut self.sim.world, at);
        } else if hand.dragging {
            self.editor.drag(&mut self.sim.world, at);
        }
        if !hand.held {
            self.editor.release();
        }
        if hand.hovered || hand.held {
            if self.editor.tool == Tool::Move {
                // A hand over what can be picked up, a closed one carrying it.
                if self.editor.carrying().is_some() {
                    ui.state.cursor_icon = CursorIcon::Grabbing;
                } else if self.sim.world.prop_at(at.x.floor() as i32, at.y.floor() as i32).is_some() {
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

    /// What stands in the world besides the map's own props: the
    /// player, the monsters, the people, the grave; and while editing,
    /// ghosts of the player on their start and of everyone else on the
    /// tile they live on.
    pub(super) fn extras(&self) -> Vec<Placed> {
        let mut extras = Vec::new();
        let middle = |x: i32, y: i32| Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
        let walk = &self.look.player;
        if self.editing {
            if let Some((x, y)) = self.sim.world.marker(Marker::PlayerStart)
                && let Some((sprite, flip)) = walk.sprite(Facing::Down, Pose::Idle)
            {
                extras.push(Placed { sprite, ground: middle(x, y), flip, tint: [GHOST; 4] });
            }
            for (kind, x, y) in self.sim.world.spawns() {
                if let Some((sprite, flip)) = self.look.monster(kind).sprite(Facing::Down, Pose::Idle) {
                    extras.push(Placed { sprite, ground: middle(x, y), flip, tint: [GHOST; 4] });
                }
            }
        } else {
            let into = (self.world_phase / TICK_SECONDS).clamp(0.0, 1.0);
            let grounds = self.monster_grounds();
            for (i, monster) in self.sim.monsters.iter().enumerate().filter(|(_, m)| m.alive()) {
                if self.sim.target() == Some(i) {
                    extras.extend(ring(self.look.target_ring, grounds[i], RING_FIGHT));
                }
                // One swing for each blow; else a walk, a stride to
                // each tile, the strides taking turns.
                let swing = Pose::swing(monster.swung_at == Some(self.sim.ticks), self.sim.monster_swing_due(i), into);
                let pose = swing.unwrap_or_else(|| Pose::at(monster.moved(), self.sim.ticks, into));
                let facing = self.monster_facing.get(i).copied().unwrap_or_default();
                if let Some((sprite, flip)) = self.look.monster(monster.kind).sprite(facing, pose) {
                    extras.push(Placed { sprite, ground: grounds[i], flip, tint: [1.0; 4] });
                }
            }
        }
        if let Some((x, y)) = self.sim.using().filter(|_| !self.editing) {
            // The rock, herb, station or grave they are set on using:
            // the grave is no prop, and no wider than a tile.
            let sprite = self.sim.world.prop_at(x, y).map_or(self.look.target_ring, |prop| self.look.ring_under(&self.sprites, prop.kind));
            extras.extend(ring(sprite, middle(x, y), RING_USE));
        }
        self.stand_people(&mut extras);
        if let (Some(grave), Some(sprite)) = (&self.sim.grave, self.look.gravestone) {
            extras.push(Placed { sprite, ground: middle(grave.tile.0, grave.tile.1), flip: false, tint: [1.0; 4] });
        }
        let into = self.phase / TICK_SECONDS;
        // One swing for each blow struck or swing at a rock done, and
        // standing ready between them.
        let player = &self.sim.player;
        let pose = Pose::swing(player.swung, self.sim.swing_due(), into).unwrap_or_else(|| Pose::at(player.moved(), player.steps, into));
        // The player is drawn in layers, each piece of what they wear
        // in its metal's colour.
        let worn = |slot| self.sim.gear.worn(slot).and_then(|item| Some((self.look.gear(slot), metal_tint(item.metal()?))));
        let worn: Vec<_> = GEAR_ORDER.into_iter().filter_map(worn).collect();
        dress(&mut extras, self.player_ground(), self.facing, pose, walk, &worn);
        extras
    }
}
