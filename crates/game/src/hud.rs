//! What floats over the world while playing: the bag, the skills, the
//! buttons that open them, and what the game has to say about what
//! just happened.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{ImageHandle, Sense, Ui};
use sim::items::SLOTS;
use sim::{Event, Item, Sim, Skill};

/// Sizes in logical pixels.
const SLOT: f64 = 75.0;
const GAP: f64 = 5.0;
const PAD: f64 = 10.0;
/// The bag is taller than wide: five across, six down.
const COLUMNS: usize = 5;
const SKILLS_WIDTH: f64 = 380.0;
const BUTTON: Vec2 = Vec2::new(130.0, 55.0);
/// How far the buttons and panels keep from the window's edge.
const MARGIN: f64 = 15.0;
/// How long a line of floating text lasts, in seconds, and how far it
/// rises in that time.
const DROP_LIFE: f64 = 1.6;
const DROP_RISE: f64 = 70.0;
/// How long a message stays at the top of the screen, and how long of
/// that it spends fading out.
const MESSAGE_LIFE: f64 = 3.5;
const MESSAGE_FADE: f64 = 0.5;
/// The most messages shown at once: older ones make way.
const MESSAGES: usize = 4;
/// The layer panels float on: over the world, under the shell's popups.
const LAYER: usize = 1;

/// A line of text and when it appeared: rising from the player (what
/// they just got) or at the top of the screen (what the game says).
struct Line {
    text: String,
    born: f64,
}

/// The items' pictures, uploaded for the interface to draw.
#[derive(Default)]
pub struct ItemPictures(pub [Option<ImageHandle>; Item::ALL.len()]);

impl ItemPictures {
    fn of(&self, item: Item) -> Option<ImageHandle> {
        self.0[Item::ALL.iter().position(|i| *i == item)?]
    }
}

#[derive(Default)]
pub struct Hud {
    pub bag_open: bool,
    pub skills_open: bool,
    drops: Vec<Line>,
    messages: Vec<Line>,
}

impl Hud {
    /// Take in what the simulation says happened at time `now`. What was
    /// got floats up from the player; the rest is said at the top of
    /// the screen.
    pub fn hear(&mut self, events: &[Event], now: f64) {
        for event in events {
            let (lines, text) = match *event {
                Event::Got { item, count } => (&mut self.drops, format!("+{count} {}", item.label())),
                Event::Xp { skill, amount } => (&mut self.drops, format!("+{amount} {} xp", skill.label())),
                Event::LevelUp { skill, level } => (&mut self.messages, format!("{} level {level}", skill.label())),
                Event::InventoryFull => (&mut self.messages, "Your bag is full".to_owned()),
                Event::TooHard { skill, level } => (&mut self.messages, format!("Needs {} level {level}", skill.label())),
                Event::Swing { .. } | Event::Spent { .. } => continue,
            };
            lines.push(Line { text, born: now });
        }
        let extra = self.messages.len().saturating_sub(MESSAGES);
        self.messages.drain(..extra);
    }

    /// Say something at the top of the screen.
    pub fn say(&mut self, text: &str, now: f64) {
        self.messages.push(Line { text: text.to_owned(), born: now });
    }

    /// Whether something here is moving, and so wants another frame.
    pub fn animating(&self, now: f64) -> bool {
        self.drops.iter().any(|d| now - d.born < DROP_LIFE) || self.messages.iter().any(|m| now - m.born < MESSAGE_LIFE)
    }

    /// The lines floating up right now, oldest first.
    #[cfg(test)]
    pub fn floating(&self) -> Vec<&str> {
        self.drops.iter().map(|d| d.text.as_str()).collect()
    }

    /// The messages at the top of the screen right now, oldest first.
    #[cfg(test)]
    pub fn said(&self) -> Vec<&str> {
        self.messages.iter().map(|m| m.text.as_str()).collect()
    }

    /// Draw over the world's area `view`. `above_player` is the point
    /// on screen just over the player's head.
    pub fn draw(&mut self, ui: &mut Ui, view: Rect, sim: &Sim, pictures: &ItemPictures, above_player: Vec2) {
        let now = ui.now();
        self.drops.retain(|d| now - d.born < DROP_LIFE);
        self.messages.retain(|m| now - m.born < MESSAGE_LIFE);
        self.draw_drops(ui, above_player, now);
        self.draw_messages(ui, view, now);

        let m = ui.m;
        let px = |v: f64| m.px(v);
        let (margin, gap) = (px(MARGIN), px(GAP));
        let corner = view.max - Vec2::new(margin, margin);
        let button = Vec2::new(px(BUTTON.x), px(BUTTON.y));
        let bar = Rect::new(corner - Vec2::new(button.x * 2.0 + gap, button.y), corner);
        ui.child(bar, LAYER, |ui| {
            ui.row(|ui| {
                if ui.button_sized("Skills", button).clicked {
                    self.skills_open = !self.skills_open;
                }
                if ui.button_sized("Bag", button).clicked {
                    self.bag_open = !self.bag_open;
                }
            });
        });

        // Panels stand on the buttons, the bag at the right and the
        // skills to its left.
        let mut right = bar.max.x;
        let floor = bar.min.y - gap;
        if self.bag_open {
            let rows = SLOTS.div_ceil(COLUMNS) as f64;
            let size = Vec2::new(px(SLOT) * COLUMNS as f64 + gap * (COLUMNS as f64 - 1.0) + px(PAD) * 2.0, px(SLOT) * rows + gap * rows + m.widget_h + px(PAD) * 2.0);
            let rect = Rect::new(Vec2::new(right - size.x, floor - size.y), Vec2::new(right, floor));
            ui.child(rect, LAYER, |ui| draw_bag(ui, rect, sim, pictures));
            right = rect.min.x - gap;
        }
        if self.skills_open {
            let rows = Skill::ALL.len() as f64;
            let size = Vec2::new(px(SKILLS_WIDTH), (m.widget_h + m.gap) * rows - m.gap + px(PAD) * 2.0);
            let rect = Rect::new(Vec2::new(right - size.x, floor - size.y), Vec2::new(right, floor));
            ui.child(rect, LAYER, |ui| {
                backdrop(ui, rect);
                ui.child(rect.shrink(px(PAD)), LAYER, |ui| {
                    for skill in Skill::ALL {
                        ui.progress(&format!("{}  {}", skill.label(), sim.skills.level(skill)), sim.skills.progress(skill));
                    }
                });
            });
        }
    }

    fn draw_drops(&self, ui: &mut Ui, above_player: Vec2, now: f64) {
        let style = ui.text_style();
        let line = style.line_height() as f64;
        for (i, drop) in self.drops.iter().enumerate() {
            let age = ((now - drop.born) / DROP_LIFE).clamp(0.0, 1.0);
            // Lines that arrive together stack; all rise and fade out.
            let together = self.drops[..i].iter().filter(|d| d.born == drop.born).count() as f64;
            let middle = above_player - Vec2::new(0.0, ui.m.px(DROP_RISE) * age + line * together);
            let rect = Rect::from_center_size(middle, Vec2::new(ui.m.px(600.0), line));
            let alpha = 1.0 - age * age;
            let nudge = Vec2::new(0.0, ui.m.px(2.0));
            ui.text_centered(&drop.text, &style, rect.translate(nudge), Color::rgba(0.0, 0.0, 0.0, 0.8 * alpha));
            ui.text_centered(&drop.text, &style, rect, Color::rgba(1.0, 1.0, 1.0, alpha));
        }
    }
}

impl Hud {
    /// What the game has to say, in big letters at the top of the
    /// screen, each on a dark plate so it reads over anything.
    fn draw_messages(&self, ui: &mut Ui, view: Rect, now: f64) {
        let style = ui.heading_style();
        let (line, pad, gap) = (style.line_height() as f64, ui.m.px(PAD), ui.m.px(GAP));
        for (i, message) in self.messages.iter().enumerate() {
            let left = MESSAGE_LIFE - (now - message.born);
            let alpha = (left / MESSAGE_FADE).clamp(0.0, 1.0);
            let width = ui.measure(&message.text, &style) + pad * 4.0;
            let top = view.min.y + ui.m.px(MARGIN) + i as f64 * (line + pad * 2.0 + gap);
            let plate = Rect::from_min_size(Vec2::new((view.center().x - width * 0.5).round(), top), Vec2::new(width, line + pad * 2.0));
            ui.draw.rounded_rect(plate, ui.m.radius, Color::rgba(0.0, 0.0, 0.0, 0.65 * alpha));
            ui.text_centered(&message.text, &style, plate, Color::rgba(1.0, 1.0, 1.0, alpha));
        }
    }
}

/// A panel's own ground: the theme's panel, with a dark edge.
fn backdrop(ui: &mut Ui, rect: Rect) {
    ui.draw.rounded_rect_gradient(rect, ui.m.radius, ui.theme.panel.top, ui.theme.panel.bottom);
    ui.draw.stroke_rect(rect, ui.m.border, ui.m.radius, ui.theme.border_dark);
}

/// The bag: every slot with what it holds and how many, and under them
/// the name of what the pointer is on.
fn draw_bag(ui: &mut Ui, rect: Rect, sim: &Sim, pictures: &ItemPictures) {
    backdrop(ui, rect);
    let (slot, gap, pad) = (ui.m.px(SLOT), ui.m.px(GAP), ui.m.px(PAD));
    let style = ui.text_style();
    let mut pointed = None;
    for (i, held) in sim.inventory.slots().iter().enumerate() {
        let (column, row) = ((i % COLUMNS) as f64, (i / COLUMNS) as f64);
        let at = rect.min + Vec2::new(pad + column * (slot + gap), pad + row * (slot + gap));
        let cell = Rect::from_min_size(at, Vec2::new(slot, slot));
        let r = ui.interact(ui.id("slot").with_index(i), cell, Sense::NONE);
        ui.recessed(cell, ui.theme.field);
        let Some(stack) = held else { continue };
        if r.hovered {
            pointed = Some(*stack);
            ui.draw.stroke_rect(cell, ui.m.border * 2.0, ui.m.radius, ui.theme.accent);
        }
        if let Some(image) = pictures.of(stack.item) {
            let room = slot - pad * 0.5;
            let scale = (room / image.width.max(1) as f64).min(room / image.height.max(1) as f64);
            let fitted = Rect::from_center_size(cell.center(), Vec2::new(image.width as f64, image.height as f64) * scale).round();
            ui.draw.image(fitted, image, 0.0, Color::WHITE);
        }
        if stack.item.stacks() {
            // The count in the corner, with a dark copy under it so it
            // reads over any picture.
            let line = style.line_height() as f64;
            let corner = Rect::new(Vec2::new(cell.min.x, cell.max.y - line), cell.max - Vec2::new(pad * 0.5, 0.0));
            let count = stack.count.to_string();
            ui.text_right(&count, &style, corner.translate(Vec2::new(0.0, ui.m.px(2.0))), Color::rgba(0.0, 0.0, 0.0, 0.85));
            ui.text_right(&count, &style, corner, ui.theme.text);
        }
    }
    if let Some(stack) = pointed {
        let rows = SLOTS.div_ceil(COLUMNS) as f64;
        let top = rect.min.y + pad + rows * (slot + gap);
        let line = Rect::new(Vec2::new(rect.min.x + pad, top), Vec2::new(rect.max.x - pad, top + ui.m.widget_h));
        ui.text_in_rect(stack.item.label(), &style, line, ui.theme.text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_was_got_floats_up_and_the_rest_is_said() {
        let mut hud = Hud::default();
        let events = [
            Event::Swing { at: (1, 1), hit: true },
            Event::Got { item: Item::CopperOre, count: 1 },
            Event::Xp { skill: Skill::Mining, amount: 12 },
            Event::LevelUp { skill: Skill::Mining, level: 2 },
            Event::Spent { at: (1, 1) },
        ];
        hud.hear(&events, 10.0);
        assert_eq!(hud.said(), vec!["Mining level 2"]);
        assert_eq!(hud.floating(), vec!["+1 Copper ore", "+12 Mining xp"]);
        hud.hear(&[Event::InventoryFull, Event::TooHard { skill: Skill::Mining, level: 20 }], 10.0);
        assert_eq!(hud.said(), vec!["Mining level 2", "Your bag is full", "Needs Mining level 20"]);
    }

    #[test]
    fn only_the_newest_few_messages_are_kept() {
        let mut hud = Hud::default();
        for level in 2..10 {
            hud.hear(&[Event::LevelUp { skill: Skill::Mining, level }], level as f64);
        }
        assert_eq!(hud.said(), vec!["Mining level 6", "Mining level 7", "Mining level 8", "Mining level 9"]);
        assert!(hud.animating(9.0) && !hud.animating(9.0 + MESSAGE_LIFE + 0.01));
    }

    #[test]
    fn floating_text_wants_frames_only_while_it_lasts() {
        let mut hud = Hud::default();
        assert!(!hud.animating(0.0));
        hud.hear(&[Event::Xp { skill: Skill::Mining, amount: 12 }], 5.0);
        assert!(hud.animating(5.0) && hud.animating(5.0 + DROP_LIFE - 0.01));
        assert!(!hud.animating(5.0 + DROP_LIFE + 0.01));
    }
}
