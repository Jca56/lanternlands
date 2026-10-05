//! The bag's panel, and the slot that it and the bank are both made of.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{Sense, Ui};
use sim::Sim;
use sim::items::{SLOTS, Stack};

use crate::drag::{Drag, Place};
use crate::hud::{ItemPictures, Press, backdrop};
use crate::tonics;

/// Sizes in logical pixels.
pub const SLOT: f64 = 75.0;
pub const GAP: f64 = 5.0;
pub const PAD: f64 = 10.0;
/// The bag is taller than wide: five across, six down.
const COLUMNS: usize = 5;

/// How big a panel of `count` slots, `columns` across, is: the slots,
/// and under them a line for the name of what the pointer is on.
pub fn grid_size(ui: &Ui, count: usize, columns: usize) -> Vec2 {
    let (slot, gap, pad) = (ui.m.px(SLOT), ui.m.px(GAP), ui.m.px(PAD));
    let rows = count.div_ceil(columns) as f64;
    Vec2::new(slot * columns as f64 + gap * (columns as f64 - 1.0) + pad * 2.0, slot * rows + gap * rows + ui.m.widget_h + pad * 2.0)
}

/// Where slot `i` of a grid `columns` across is, with the grid's first
/// slot at `first`.
pub fn cell(ui: &Ui, first: Vec2, i: usize, columns: usize) -> Rect {
    let (slot, gap) = (ui.m.px(SLOT), ui.m.px(GAP));
    let at = first + Vec2::new((i % columns) as f64 * (slot + gap), (i / columns) as f64 * (slot + gap));
    Rect::from_min_size(at, Vec2::new(slot, slot))
}

/// One slot of a grid of things: its recess, the picture of what it
/// holds, and an edge round it while it is `lit`: the pointer is on
/// what it holds, or something carried is over it. What is being
/// carried off is drawn `faint` where it was.
pub fn slot(ui: &mut Ui, cell: Rect, held: Option<Stack>, pictures: &ItemPictures, lit: bool, faint: bool) {
    ui.recessed(cell, ui.theme.field);
    if lit {
        ui.draw.stroke_rect(cell, ui.m.border * 2.0, ui.m.radius, ui.theme.accent);
    }
    let Some(stack) = held else { return };
    if let Some(image) = pictures.of(stack.item) {
        let room = cell.width() - ui.m.px(PAD) * 0.5;
        let scale = (room / image.width.max(1) as f64).min(room / image.height.max(1) as f64);
        let fitted = Rect::from_center_size(cell.center(), Vec2::new(image.width as f64, image.height as f64) * scale).round();
        ui.draw.image(fitted, image, 0.0, Color::WHITE.with_alpha(if faint { 0.3 } else { 1.0 }));
    }
}

/// How many a slot holds, in its corner, with a dark copy under it so
/// it reads over any picture.
pub fn count(ui: &mut Ui, cell: Rect, count: u32) {
    let style = ui.text_style();
    let line = style.line_height() as f64;
    let corner = Rect::new(Vec2::new(cell.min.x, cell.max.y - line), cell.max - Vec2::new(ui.m.px(PAD) * 0.5, 0.0));
    let count = count.to_string();
    ui.text_right(&count, &style, corner.translate(Vec2::new(0.0, ui.m.px(2.0))), Color::rgba(0.0, 0.0, 0.0, 0.85));
    ui.text_right(&count, &style, corner, Color::WHITE);
}

/// The line along the bottom of a grid's panel `rect`, where the name
/// of what the pointer is on is written.
pub fn name_line(ui: &Ui, rect: Rect) -> Rect {
    let pad = ui.m.px(PAD);
    Rect::new(Vec2::new(rect.min.x + pad, rect.max.y - pad - ui.m.widget_h), rect.max - Vec2::splat(pad))
}

/// How big the bag's panel is.
pub fn size(ui: &Ui) -> Vec2 {
    grid_size(ui, SLOTS, COLUMNS)
}

/// The bag, in `rect`: every slot with what it holds and how many, and
/// under them the name of what the pointer is on. Gives what a press on
/// something in it asks for: a plain press on what can be used (gear to
/// put on, a potion to drink, and at the bank anything at all, to put
/// in), or a right press on anything. Anything in it can be carried
/// off, and anything carried can be put down in it.
pub fn draw(ui: &mut Ui, rect: Rect, sim: &Sim, pictures: &ItemPictures, into: f64, drag: &mut Drag) -> Option<Press> {
    backdrop(ui, rect);
    let first = rect.min + Vec2::splat(ui.m.px(PAD));
    let (mut pressed, mut pointed) = (None, None);
    for (i, held) in sim.inventory.slots().iter().enumerate() {
        let cell = cell(ui, first, i, COLUMNS);
        let usable = held.is_some_and(|stack| sim.banking() || stack.item.slot().is_some() || stack.item.potion().is_some());
        let r = ui.interact(ui.id("slot").with_index(i), cell, if held.is_some() { Sense::DRAG } else { Sense::NONE });
        let place = Place::Bag(i);
        drag.slot(ui, place, cell, held.map(|stack| stack.item), &r);
        slot(ui, cell, *held, pictures, (r.hovered && held.is_some()) || drag.aimed_at(ui, cell), drag.lifted_from(place));
        let Some(stack) = held else { continue };
        if r.clicked && usable && !drag.lifted_from(place) {
            pressed = Some(Press::Bag(i));
        }
        if r.hovered {
            pointed = Some(*stack);
            if ui.state.right_pressed {
                pressed = Some(Press::BagMenu(i));
            }
        }
        tonics::draw_wait(ui, cell, sim, stack.item, into);
        if stack.item.stacks() {
            count(ui, cell, stack.count);
        }
    }
    if let Some(stack) = pointed {
        let style = ui.text_style();
        let line = name_line(ui, rect);
        ui.text_in_rect(stack.item.label(), &style, line, ui.theme.text);
    }
    pressed
}
