//! The hotbar: six big buttons along the bottom of the screen, each an
//! ability, worked with a press of the button or of its number key.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{ImageHandle, Sense, Ui};
use sim::{Ability, Sim, TICK_SECONDS};

pub const SLOTS: usize = 6;
/// What is on each button, left to right. The same for everyone, for
/// now: there is nothing yet to choose between.
pub const BAR: [Option<Ability>; SLOTS] = [Some(Ability::HeavyStrike), Some(Ability::Flurry), Some(Ability::Kick), Some(Ability::Guard), None, None];

/// Sizes in logical pixels.
const SLOT: f64 = 90.0;
const GAP: f64 = 10.0;
const PAD: f64 = 10.0;
const MARGIN: f64 = 15.0;
/// The corner chip a button's key is written on.
const CHIP: f64 = 30.0;
/// The layer the bar floats on: over the world, with the panels.
const LAYER: usize = 1;

/// What a button with an ability on it stands on: light, so the
/// drawings' dark outlines read.
const PLATE: Color = Color::hex(0xc9b68a);
const PLATE_HOVER: Color = Color::hex(0xdccb9f);
const EDGE: Color = Color::hex(0x271a1c);
/// Round a button asked for and waiting, and round a guard that is up.
const WAITING: Color = Color::hex(0xf2ead8);
const GUARD_UP: Color = Color::hex(0x8fb6ff);

/// The abilities' pictures, uploaded for the interface to draw.
#[derive(Default)]
pub struct AbilityPictures(pub [Option<ImageHandle>; Ability::ALL.len()]);

impl AbilityPictures {
    fn of(&self, ability: Ability) -> Option<ImageHandle> {
        self.0[Ability::ALL.iter().position(|a| *a == ability)?]
    }
}

/// Where the bar is in the world's area `view`.
pub fn rect(ui: &Ui, view: Rect) -> Rect {
    let (slot, gap, pad) = (ui.m.px(SLOT), ui.m.px(GAP), ui.m.px(PAD));
    let size = Vec2::new(slot * SLOTS as f64 + gap * (SLOTS as f64 - 1.0) + pad * 2.0, slot + pad * 2.0);
    let middle = Vec2::new(view.center().x, view.max.y - ui.m.px(MARGIN) - size.y * 0.5);
    Rect::from_center_size(middle, size).round()
}

/// Seconds until an ability can be used again, `into` the world's tick.
pub fn seconds_left(sim: &Sim, ability: Ability, into: f64) -> f64 {
    match sim.cooldown_left(ability) {
        0 => 0.0,
        ticks => (ticks as f64 - into.clamp(0.0, 1.0)) * TICK_SECONDS,
    }
}

/// Draw the bar. Gives the ability whose button was pressed, if one
/// was. `into` is how far into the world's tick this frame is, 0 to 1.
pub fn draw(ui: &mut Ui, view: Rect, sim: &Sim, pictures: &AbilityPictures, into: f64) -> Option<Ability> {
    let bar = rect(ui, view);
    let (slot, gap, pad) = (ui.m.px(SLOT), ui.m.px(GAP), ui.m.px(PAD));
    let mut pressed = None;
    ui.child(bar, LAYER, |ui| {
        // The bar's own ground. Floating on its layer, all of it keeps
        // presses from the world behind, between the buttons too.
        ui.draw.rounded_rect(bar, ui.m.radius, Color::rgba(0.0, 0.0, 0.0, 0.45));
        let (plain, big) = (ui.text_style(), ui.heading_style());
        for (i, held) in BAR.iter().enumerate() {
            let cell = Rect::from_min_size(bar.min + Vec2::new(pad + i as f64 * (slot + gap), pad), Vec2::new(slot, slot));
            let r = ui.interact(ui.id("ability").with_index(i), cell, Sense::CLICK);
            let Some(ability) = *held else {
                ui.recessed(cell, ui.theme.field);
                continue;
            };
            let left = seconds_left(sim, ability, into);
            let ready = left <= 0.0;
            if r.pressed {
                pressed = Some(ability);
            }
            ui.draw.rounded_rect(cell, ui.m.radius, if r.hovered && ready { PLATE_HOVER } else { PLATE });
            if let Some(image) = pictures.of(ability) {
                let room = slot - pad;
                let scale = (room / image.width.max(1) as f64).min(room / image.height.max(1) as f64);
                let fitted = Rect::from_center_size(cell.center(), Vec2::new(image.width as f64, image.height as f64) * scale).round();
                ui.draw.image(fitted, image, 0.0, Color::WHITE);
            }
            if !ready {
                // Dark over the whole button, darker over the share of
                // the wait still to go, and the seconds left written big.
                let share = (left / (ability.cooldown() as f64 * TICK_SECONDS)).clamp(0.0, 1.0);
                ui.draw.rounded_rect(cell, ui.m.radius, Color::rgba(0.0, 0.0, 0.0, 0.45));
                let (_, still) = cell.split_y(cell.max.y - (cell.height() * share).round());
                ui.draw.rounded_rect(still, ui.m.radius, Color::rgba(0.0, 0.0, 0.0, 0.4));
                let text = format!("{}", left.ceil() as u32);
                ui.text_centered(&text, &big, cell.translate(Vec2::new(0.0, ui.m.px(2.0))), Color::rgba(0.0, 0.0, 0.0, 0.85));
                ui.text_centered(&text, &big, cell, Color::WHITE);
            }
            let edge = if ability == Ability::Guard && sim.guard_left() > 0 {
                Some(GUARD_UP)
            } else if sim.queued(ability) {
                Some(WAITING)
            } else {
                None
            };
            match edge {
                Some(color) => ui.draw.stroke_rect(cell, ui.m.px(5.0), ui.m.radius, color),
                None => ui.draw.stroke_rect(cell, ui.m.border.max(2.0), ui.m.radius, EDGE),
            }
            // Its key, on a dark chip in the corner.
            let chip = Rect::from_min_size(cell.min, Vec2::new(ui.m.px(CHIP), ui.m.px(CHIP)));
            ui.draw.rounded_rect(chip, ui.m.radius, Color::rgba(0.0, 0.0, 0.0, 0.75));
            ui.text_centered(&(i + 1).to_string(), &plain, chip, Color::WHITE);
        }
    });
    pressed
}
