//! The panel of what the player wears: seven slots laid out like a
//! body, and what it all adds up to. A press on something worn takes
//! it off.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{Sense, Ui};
use sim::{Sim, Slot};

use crate::hotbar::PLATE;
use crate::hud::ItemPictures;

/// Sizes in logical pixels.
const SLOT: f64 = 90.0;
const GAP: f64 = 5.0;
const PAD: f64 = 10.0;
/// The lines under the slots: what the gear adds.
const LINES: usize = 3;

/// Where each slot sits, as (column, row) of a three by four grid: the
/// head over the body, a hand's thing to either side of it, the legs
/// and feet under.
const PLACES: [(Slot, usize, usize); 7] = [(Slot::Head, 1, 0), (Slot::Weapon, 0, 1), (Slot::Body, 1, 1), (Slot::Shield, 2, 1), (Slot::Hands, 0, 2), (Slot::Legs, 1, 2), (Slot::Feet, 1, 3)];

/// How big the panel is.
pub fn size(ui: &Ui) -> Vec2 {
    let (slot, gap, pad) = (ui.m.px(SLOT), ui.m.px(GAP), ui.m.px(PAD));
    let line = ui.text_style().line_height() as f64;
    Vec2::new(slot * 3.0 + gap * 2.0 + pad * 2.0, slot * 4.0 + gap * 3.0 + pad * 3.0 + line * LINES as f64)
}

/// Draw the panel in `rect`. Gives the slot whose gear was pressed, to
/// be taken off.
pub fn draw(ui: &mut Ui, rect: Rect, sim: &Sim, pictures: &ItemPictures) -> Option<Slot> {
    let (slot, gap, pad) = (ui.m.px(SLOT), ui.m.px(GAP), ui.m.px(PAD));
    let style = ui.text_style();
    let mut off = None;
    for (place, column, row) in PLACES {
        let at = rect.min + Vec2::new(pad + column as f64 * (slot + gap), pad + row as f64 * (slot + gap));
        let cell = Rect::from_min_size(at, Vec2::new(slot, slot));
        let worn = sim.gear.worn(place);
        let r = ui.interact(ui.id("gear").with(place.name()), cell, if worn.is_some() { Sense::CLICK } else { Sense::NONE });
        let Some(item) = worn else {
            // An empty slot says what goes in it.
            ui.recessed(cell, ui.theme.field);
            ui.text_centered(place.label(), &style, cell, ui.theme.text.with_alpha(0.4));
            continue;
        };
        ui.draw.rounded_rect(cell, ui.m.radius, PLATE);
        if let Some(image) = pictures.of(item) {
            ui.draw.image(cell.shrink(gap), image, 0.0, Color::WHITE);
        }
        let edge = if r.hovered { (ui.m.border * 2.0, ui.theme.accent) } else { (ui.m.border.max(2.0), ui.theme.border_dark) };
        ui.draw.stroke_rect(cell, edge.0, ui.m.radius, edge.1);
        if r.clicked {
            off = Some(place);
        }
    }
    // What it all adds up to.
    let bonus = sim.gear.bonus();
    let line = style.line_height() as f64;
    let top = rect.min.y + pad * 2.0 + slot * 4.0 + gap * 3.0;
    let lines = [("Accuracy", bonus.accuracy as u32), ("Damage", bonus.damage), ("Armour", bonus.armour as u32)];
    for (i, (what, much)) in lines.into_iter().enumerate() {
        let row = Rect::from_min_size(Vec2::new(rect.min.x + pad, top + i as f64 * line), Vec2::new(rect.width() - pad * 2.0, line));
        ui.text_in_rect(what, &style, row, ui.theme.text);
        ui.text_right(&format!("+{much}"), &style, row, ui.theme.text);
    }
    off
}
