//! The world's area of the window: the game while playing, and the
//! held tool's workbench while the map editor is open.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{AreaCx, CursorIcon, Dialog, Key, Response, Sense, ShellRequest, Ui};
use sim::{Marker, TICK_SECONDS};

use super::Game;
use crate::anim::{Facing, Pose};
use crate::editor::Tool;
use crate::scene::Placed;

/// How much one notch of the wheel zooms.
const ZOOM_PER_NOTCH: f64 = 1.15;
/// How fast the keys move the view, in logical pixels a second.
const KEY_PAN_SPEED: f64 = 1000.0;
/// The longest step the keys take at once, however long the last frame
/// took.
const LONGEST_STEP: f64 = 0.1;
/// How quickly the view closes on the player: the share of the gap it
/// covers each second is all but `e` to minus this.
const FOLLOW_RATE: f64 = 8.0;
/// Closer to the player than this, in tiles, the view is on them.
const FOLLOW_SETTLED: f64 = 0.002;
/// The ring on the tile the player is walking to.
const GOAL_RING: Color = Color::rgba(0.757, 0.608, 0.290, 0.9);
/// How see-through the editor's ghost of the player's start is.
const GHOST: f32 = 0.5;

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
        let dt = self.clock.replace(now).map_or(0.0, |last| (now - last).clamp(0.0, LONGEST_STEP));
        let hand = ui.interact(ui.id("world"), self.view, Sense::DRAG);
        self.zoom(ui, hand.hovered);
        if self.editing {
            self.steer(ui, hand.hovered);
            self.edit(ui, &hand);
        } else {
            self.editor.release();
            self.play(ui, &hand, dt);
        }
    }

    /// Playing: a press walks the player there, the clock ticks while
    /// they have somewhere to go, and the view glides after them.
    fn play(&mut self, ui: &mut Ui, hand: &Response, dt: f64) {
        if hand.pressed {
            let at = self.camera.to_world(ui.state.pointer, self.view, self.scale);
            self.sim.walk_to((at.x.floor() as i32, at.y.floor() as i32));
        }
        let busy = self.sim.player.goal().is_some() || self.sim.player.moved();
        if busy {
            self.phase += dt;
            while self.phase >= TICK_SECONDS {
                self.phase -= TICK_SECONDS;
                self.sim.tick();
                let (from, tile) = (self.sim.player.from, self.sim.player.tile);
                self.facing = Facing::of_step(tile.0 - from.0, tile.1 - from.1).unwrap_or(self.facing);
            }
        }
        if self.sim.player.goal().is_none() && !self.sim.player.moved() {
            self.phase = TICK_SECONDS;
        }

        let target = self.player_ground();
        let gap = target - self.camera.center;
        let following = gap.length() > FOLLOW_SETTLED;
        self.camera.center = if following { self.camera.center + gap * (1.0 - (-FOLLOW_RATE * dt).exp()) } else { target };
        if busy || following {
            ui.state.request_redraw_after(0.0);
        }

        if let Some((x, y)) = self.sim.player.goal() {
            let middle = self.camera.to_screen(Vec2::new(x as f64 + 0.5, y as f64 + 0.5), self.view, self.scale);
            let tile_px = self.camera.tile_px(self.scale);
            ui.draw.ring(middle, tile_px * 0.3, (tile_px * 0.06).max(2.0), GOAL_RING);
        }
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

    /// What stands in the world besides the map's own props: the player
    /// and, while editing, a ghost of them on their start.
    pub(super) fn extras(&self) -> Vec<Placed> {
        let mut extras = Vec::new();
        let walk = &self.look.player;
        if self.editing
            && let Some((x, y)) = self.sim.world.marker(Marker::PlayerStart)
            && let Some((sprite, flip)) = walk.sprite(Facing::Down, Pose::Idle)
        {
            extras.push(Placed { sprite, ground: Vec2::new(x as f64 + 0.5, y as f64 + 0.5), flip, tint: [GHOST; 4] });
        }
        let into = self.phase / TICK_SECONDS;
        let pose = Pose::at(self.sim.player.moved(), self.sim.player.steps, into);
        if let Some((sprite, flip)) = walk.sprite(self.facing, pose) {
            extras.push(Placed { sprite, ground: self.player_ground(), flip, tint: [1.0; 4] });
        }
        extras
    }
}
