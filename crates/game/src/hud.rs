//! What floats over the world while playing: the bag, the skills, the
//! buttons that open them, and what the game has to say about what
//! just happened.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{ImageHandle, Sense, Ui};
use sim::items::SLOTS;
use sim::{Event, Item, Sim, Skill, Slot, Who};

use crate::{gear_panel, hotbar, tonics};

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

/// The player's health bar, in logical pixels.
const HEALTH: Vec2 = Vec2::new(300.0, 40.0);
/// A monster's health bar: how wide, in tiles, how tall, in logical
/// pixels, and how far over its feet, in tiles.
const BAR_WIDTH: f64 = 1.0;
const BAR_HEIGHT: f64 = 15.0;
const BAR_OVER: f64 = 1.3;
/// How far over a monster's feet a number rises from, in tiles: clear
/// of its bar.
const HEAD_OVER: f64 = 1.6;
/// The same for the player, who is taller: their bar, what rises over
/// it while it shows, and what rises from their head while it does not.
const PLAYER_BAR_OVER: f64 = 1.6;
const PLAYER_HEAD_OVER: f64 = 1.9;
const PLAYER_HEAD: f64 = 1.45;

const HEALTH_LEFT: Color = Color::hex(0x8fce5c);
const HEALTH_LOST: Color = Color::hex(0x7a2e2a);
const PLAIN: Color = Color::WHITE;
const HURT: Color = Color::hex(0xff4a3d);
/// Health given back: lighter than the grass it floats over.
const HEALED: Color = Color::hex(0xc8f7a0);
const MISSED: Color = Color::hex(0xb9c2c8);

/// A line of text and when it appeared: rising over someone's head
/// (what they just got, what a blow did) or at the top of the screen
/// (what the game says).
struct Line {
    text: String,
    born: f64,
    /// The point of the world it rises from, in tiles; the player's
    /// head when `None`.
    at: Option<Vec2>,
    color: Color,
    /// What a blow did is written big; the rest in the plain size.
    big: bool,
}

/// Where things are on screen this frame, for what is drawn over them.
pub struct Sight<'a> {
    /// Where the player's feet are drawn, in tiles.
    pub player: Vec2,
    /// A point of the world, in tiles, to where it is on screen.
    pub to_screen: &'a dyn Fn(Vec2) -> Vec2,
    /// Where each monster's feet are drawn, in tiles.
    pub monsters: &'a [Vec2],
    /// Physical pixels a tile is across.
    pub tile_px: f64,
    /// How far into the world's tick this frame is, 0 to 1.
    pub into: f64,
}

/// The items' pictures, uploaded for the interface to draw.
#[derive(Default)]
pub struct ItemPictures(pub [Option<ImageHandle>; Item::ALL.len()]);

impl ItemPictures {
    pub fn of(&self, item: Item) -> Option<ImageHandle> {
        self.0[Item::ALL.iter().position(|i| *i == item)?]
    }
}

/// What a press on the bag or the gear asks for: what is in a slot of
/// the bag used (gear put on, a potion drunk), or what is worn in a
/// slot taken off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Bag(usize),
    Worn(Slot),
}

#[derive(Default)]
pub struct Hud {
    pub bag_open: bool,
    pub skills_open: bool,
    pub gear_open: bool,
    drops: Vec<Line>,
    messages: Vec<Line>,
}

impl Hud {
    /// Take in what the simulation says happened at time `now`. What
    /// was got and what blows did float up over whoever it happened to;
    /// the rest is said at the top of the screen.
    pub fn hear(&mut self, events: &[Event], sim: &Sim, now: f64) {
        // Over a monster's head, wherever it stands.
        let over = |who: Who| match who {
            Who::Player => None,
            Who::Monster(i) => sim.monsters.get(i).map(|m| Vec2::new(m.tile.0 as f64 + 0.5, m.tile.1 as f64 + 0.5 - HEAD_OVER)),
        };
        for event in events {
            let (text, at, color, big) = match *event {
                Event::Got { item, count } => (format!("+{count} {}", item.label()), None, PLAIN, false),
                Event::Hit { on: Who::Player, damage } => (format!("-{damage}"), None, HURT, true),
                Event::Hit { on, damage } => (damage.to_string(), over(on), PLAIN, true),
                Event::Miss { on } => ("miss".to_owned(), over(on), MISSED, true),
                // A potion that healed says how much; the others, their name.
                Event::Drank { healed, .. } if healed > 0 => (format!("+{healed}"), None, HEALED, true),
                Event::Drank { potion, .. } => (potion.item().label().to_owned(), None, PLAIN, false),
                Event::Blocked => ("block".to_owned(), None, MISSED, true),
                Event::Guarding => ("guard".to_owned(), None, PLAIN, true),
                Event::Stunned { monster } => ("stunned".to_owned(), over(Who::Monster(monster)), PLAIN, true),
                Event::NoTarget => {
                    self.say("No target", now);
                    continue;
                }
                Event::LevelUp { skill, level } => {
                    self.say(&format!("{} level {level}", skill.label()), now);
                    continue;
                }
                Event::InventoryFull => {
                    self.say("Your bag is full", now);
                    continue;
                }
                Event::TooHard { skill, level } => {
                    self.say(&format!("Needs {} level {level}", skill.label()), now);
                    continue;
                }
                Event::Died => {
                    // They wake somewhere else: what floated stays behind.
                    self.drops.clear();
                    self.say("You died", now);
                    continue;
                }
                Event::GraveLost => {
                    self.say("Your old grave crumbled", now);
                    continue;
                }
                Event::GraveOpened { all } => {
                    self.say(if all { "You have your things back" } else { "Your bag is full: the rest waits in the grave" }, now);
                    continue;
                }
                // Experience is not shown as it comes: it hid the blows' numbers.
                Event::Xp { .. } | Event::AtStation { .. } | Event::Swing { .. } | Event::Spent { .. } | Event::Slain { .. } => continue,
            };
            self.drops.push(Line { text, born: now, at, color, big });
        }
        let extra = self.messages.len().saturating_sub(MESSAGES);
        self.messages.drain(..extra);
    }

    /// Say something at the top of the screen.
    pub fn say(&mut self, text: &str, now: f64) {
        self.messages.push(Line { text: text.to_owned(), born: now, at: None, color: PLAIN, big: true });
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

    /// Draw over the world's area `view`. Gives what a press on the bag
    /// or the gear asked for, if one did.
    pub fn draw(&mut self, ui: &mut Ui, view: Rect, sim: &Sim, pictures: &ItemPictures, sight: &Sight) -> Option<Press> {
        let now = ui.now();
        self.drops.retain(|d| now - d.born < DROP_LIFE);
        self.messages.retain(|m| now - m.born < MESSAGE_LIFE);
        draw_monster_bars(ui, sim, sight);
        let hurt = player_bar_shows(sim);
        if hurt {
            draw_bar_over(ui, sight, sight.player, PLAYER_BAR_OVER, sim.player.health as f64 / sim.max_health().max(1) as f64);
        }
        // Over their head: clear of their bar while it shows.
        let above_player = (sight.to_screen)(sight.player - Vec2::new(0.0, if hurt { PLAYER_HEAD_OVER } else { PLAYER_HEAD }));
        self.draw_drops(ui, sight, above_player, now);
        let health = draw_health(ui, view, sim);
        tonics::draw_effects(ui, Vec2::new(health.min.x, health.max.y + ui.m.px(GAP)), sim, pictures, sight.into);
        self.draw_messages(ui, view, now);

        let m = ui.m;
        let px = |v: f64| m.px(v);
        let (margin, gap) = (px(MARGIN), px(GAP));
        let corner = view.max - Vec2::new(margin, margin);
        let button = Vec2::new(px(BUTTON.x), px(BUTTON.y));
        let bar = Rect::new(corner - Vec2::new(button.x * 3.0 + gap * 2.0, button.y), corner);
        ui.child(bar, LAYER, |ui| {
            ui.row(|ui| {
                if ui.button_sized("Gear", button).clicked {
                    self.gear_open = !self.gear_open;
                }
                if ui.button_sized("Skills", button).clicked {
                    self.skills_open = !self.skills_open;
                }
                if ui.button_sized("Bag", button).clicked {
                    self.bag_open = !self.bag_open;
                }
            });
        });

        // Panels stand on the buttons: the bag at the right, the skills
        // to its left, the gear to the left of those.
        // A panel wide enough to reach over the hotbar stands on that
        // instead, so neither hides the other.
        let mut wear = None;
        let mut right = bar.max.x;
        let hot = hotbar::rect(ui, view);
        let stand = |right: f64, width: f64| {
            let floor = if right - width < hot.max.x + gap { hot.min.y.min(bar.min.y) } else { bar.min.y } - gap;
            move |size: Vec2| Rect::new(Vec2::new(right - size.x, floor - size.y), Vec2::new(right, floor))
        };
        if self.bag_open {
            let rows = SLOTS.div_ceil(COLUMNS) as f64;
            let size = Vec2::new(px(SLOT) * COLUMNS as f64 + gap * (COLUMNS as f64 - 1.0) + px(PAD) * 2.0, px(SLOT) * rows + gap * rows + m.widget_h + px(PAD) * 2.0);
            let rect = stand(right, size.x)(size);
            ui.child(rect, LAYER, |ui| wear = draw_bag(ui, rect, sim, pictures, sight.into).map(Press::Bag));
            right = rect.min.x - gap;
        }
        if self.skills_open {
            let rows = Skill::ALL.len() as f64;
            let size = Vec2::new(px(SKILLS_WIDTH), (m.widget_h + m.gap) * rows - m.gap + px(PAD) * 2.0);
            let rect = stand(right, size.x)(size);
            ui.child(rect, LAYER, |ui| {
                backdrop(ui, rect);
                ui.child(rect.shrink(px(PAD)), LAYER, |ui| {
                    for skill in Skill::ALL {
                        ui.progress(&format!("{}  {}", skill.label(), sim.skills.level(skill)), sim.skills.progress(skill));
                    }
                });
            });
            right = rect.min.x - gap;
        }
        if self.gear_open {
            let size = gear_panel::size(ui);
            let rect = stand(right, size.x)(size);
            ui.child(rect, LAYER, |ui| {
                backdrop(ui, rect);
                wear = gear_panel::draw(ui, rect, sim, pictures).map(Press::Worn).or(wear);
            });
        }
        wear
    }

    fn draw_drops(&self, ui: &mut Ui, sight: &Sight, above_player: Vec2, now: f64) {
        let (plain, big) = (ui.text_style(), ui.heading_style());
        let line = plain.line_height() as f64;
        for (i, drop) in self.drops.iter().enumerate() {
            let style = if drop.big { &big } else { &plain };
            let age = ((now - drop.born) / DROP_LIFE).clamp(0.0, 1.0);
            // Lines that arrive together over the same head stack; all
            // rise and fade out.
            let together = self.drops[..i].iter().filter(|d| d.born == drop.born && d.at == drop.at).count() as f64;
            let from = drop.at.map_or(above_player, |at| (sight.to_screen)(at));
            let middle = from - Vec2::new(0.0, ui.m.px(DROP_RISE) * age + line * together);
            let rect = Rect::from_center_size(middle, Vec2::new(ui.m.px(600.0), style.line_height() as f64));
            let alpha = 1.0 - age * age;
            let nudge = Vec2::new(0.0, ui.m.px(2.0));
            ui.text_centered(&drop.text, style, rect.translate(nudge), Color::rgba(0.0, 0.0, 0.0, 0.8 * alpha));
            ui.text_centered(&drop.text, style, rect, drop.color.with_alpha(alpha));
        }
    }

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

/// A bar filled to `share` of its width: what is left over what is lost.
fn bar(ui: &mut Ui, rect: Rect, share: f64) {
    let (left, _) = rect.split_x(rect.min.x + (rect.width() * share.clamp(0.0, 1.0)).round());
    ui.draw.rect(rect, HEALTH_LOST);
    ui.draw.rect(left, HEALTH_LEFT);
    ui.draw.stroke_rect(rect, ui.m.border.max(2.0), 0.0, ui.theme.border_dark);
}

/// The player's health, in the top-left corner: a bar, and the numbers
/// on it. Gives where the bar is.
fn draw_health(ui: &mut Ui, view: Rect, sim: &Sim) -> Rect {
    let margin = ui.m.px(MARGIN);
    let rect = Rect::from_min_size(view.min + Vec2::new(margin, margin), Vec2::new(ui.m.px(HEALTH.x), ui.m.px(HEALTH.y))).round();
    let (health, max) = (sim.player.health, sim.max_health());
    bar(ui, rect, health as f64 / max.max(1) as f64);
    let style = ui.text_style();
    let text = format!("{health} / {max}");
    ui.text_centered(&text, &style, rect.translate(Vec2::new(0.0, ui.m.px(2.0))), Color::rgba(0.0, 0.0, 0.0, 0.8));
    ui.text_centered(&text, &style, rect, Color::WHITE);
    rect
}

/// A bar over every monster that is hurt or after the player.
fn draw_monster_bars(ui: &mut Ui, sim: &Sim, sight: &Sight) {
    for (monster, &feet) in sim.monsters.iter().zip(sight.monsters) {
        let max = monster.kind.stats().health;
        if !monster.alive() || (monster.health >= max && !monster.angry) {
            continue;
        }
        draw_bar_over(ui, sight, feet, BAR_OVER, monster.health as f64 / max.max(1) as f64);
    }
}

/// A health bar `over` (in tiles) someone's feet, `share` of it left.
fn draw_bar_over(ui: &mut Ui, sight: &Sight, feet: Vec2, over: f64, share: f64) {
    let middle = (sight.to_screen)(feet - Vec2::new(0.0, over));
    let rect = Rect::from_center_size(middle, Vec2::new(sight.tile_px * BAR_WIDTH, ui.m.px(BAR_HEIGHT))).round();
    bar(ui, rect, share);
}

/// Whether the player has a bar over their own head: while they are
/// hurt or in a fight, like a monster's.
pub fn player_bar_shows(sim: &Sim) -> bool {
    sim.in_combat() || sim.player.health < sim.max_health()
}

/// A panel's own ground: the theme's panel, with a dark edge.
fn backdrop(ui: &mut Ui, rect: Rect) {
    ui.draw.rounded_rect_gradient(rect, ui.m.radius, ui.theme.panel.top, ui.theme.panel.bottom);
    ui.draw.stroke_rect(rect, ui.m.border, ui.m.radius, ui.theme.border_dark);
}

/// The bag: every slot with what it holds and how many, and under them
/// the name of what the pointer is on.
/// The bag. Gives the slot of something that was pressed and can be
/// used: gear to put on, a potion to drink.
fn draw_bag(ui: &mut Ui, rect: Rect, sim: &Sim, pictures: &ItemPictures, into: f64) -> Option<usize> {
    backdrop(ui, rect);
    let mut put_on = None;
    let (slot, gap, pad) = (ui.m.px(SLOT), ui.m.px(GAP), ui.m.px(PAD));
    let style = ui.text_style();
    let mut pointed = None;
    for (i, held) in sim.inventory.slots().iter().enumerate() {
        let (column, row) = ((i % COLUMNS) as f64, (i / COLUMNS) as f64);
        let at = rect.min + Vec2::new(pad + column * (slot + gap), pad + row * (slot + gap));
        let cell = Rect::from_min_size(at, Vec2::new(slot, slot));
        let usable = held.is_some_and(|stack| stack.item.slot().is_some() || stack.item.potion().is_some());
        let r = ui.interact(ui.id("slot").with_index(i), cell, if usable { Sense::CLICK } else { Sense::NONE });
        ui.recessed(cell, ui.theme.field);
        let Some(stack) = held else { continue };
        if r.clicked {
            put_on = Some(i);
        }
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
        tonics::draw_wait(ui, cell, sim, stack.item, into);
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
    put_on
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet() -> Sim {
        Sim::new(sim::World::new())
    }

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
        hud.hear(&events, &quiet(), 10.0);
        assert_eq!(hud.said(), vec!["Mining level 2"]);
        assert_eq!(hud.floating(), vec!["+1 Copper ore"], "what was got floats; experience does not");
        hud.hear(&[Event::InventoryFull, Event::TooHard { skill: Skill::Mining, level: 20 }], &quiet(), 10.0);
        assert_eq!(hud.said(), vec!["Mining level 2", "Your bag is full", "Needs Mining level 20"]);
    }

    #[test]
    fn blows_float_over_whoever_took_them_and_dying_is_said() {
        let mut world = sim::World::new();
        world.set_terrain(4, 4, sim::Terrain::Grass);
        world.set_spawn(4, 4, Some(sim::MonsterKind::Goblin));
        let sim = Sim::new(world);
        let mut hud = Hud::default();
        hud.hear(&[Event::Hit { on: Who::Monster(0), damage: 3 }, Event::Miss { on: Who::Monster(0) }, Event::Hit { on: Who::Player, damage: 2 }, Event::Miss { on: Who::Player }], &sim, 1.0);
        assert_eq!(hud.floating(), vec!["3", "miss", "-2", "miss"]);
        let over_goblin = Some(Vec2::new(4.5, 4.5 - HEAD_OVER));
        assert_eq!(hud.drops.iter().map(|d| d.at).collect::<Vec<_>>(), vec![over_goblin, over_goblin, None, None]);
        assert_eq!(hud.drops[2].color, HURT);
        hud.hear(&[Event::Slain { monster: 0 }, Event::Died, Event::GraveLost, Event::GraveOpened { all: true }], &sim, 1.0);
        assert_eq!(hud.said(), vec!["You died", "Your old grave crumbled", "You have your things back"]);
    }

    #[test]
    fn only_the_newest_few_messages_are_kept() {
        let mut hud = Hud::default();
        for level in 2..10 {
            hud.hear(&[Event::LevelUp { skill: Skill::Mining, level }], &quiet(), level as f64);
        }
        assert_eq!(hud.said(), vec!["Mining level 6", "Mining level 7", "Mining level 8", "Mining level 9"]);
        assert!(hud.animating(9.0) && !hud.animating(9.0 + MESSAGE_LIFE + 0.01));
    }

    #[test]
    fn the_players_own_bar_shows_only_while_they_are_hurt_or_fighting() {
        let mut sim = quiet();
        assert!(!player_bar_shows(&sim));
        sim.player.health -= 1;
        assert!(player_bar_shows(&sim));
    }

    #[test]
    fn floating_text_wants_frames_only_while_it_lasts() {
        let mut hud = Hud::default();
        assert!(!hud.animating(0.0));
        hud.hear(&[Event::Got { item: Item::CopperOre, count: 1 }], &quiet(), 5.0);
        assert!(hud.animating(5.0) && hud.animating(5.0 + DROP_LIFE - 0.01));
        assert!(!hud.animating(5.0 + DROP_LIFE + 0.01));
    }
}
