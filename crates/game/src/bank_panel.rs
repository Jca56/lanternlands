//! The bank's panel: open for as long as the player stands at a bank
//! chest. Every slot of the bank with what it holds and how many; a
//! press takes one out, a right press asks for the menu of how many,
//! and a button puts the whole bag in.

use lntrn_math::{Rect, Vec2};
use lntrn_ui::{Sense, Ui};
use sim::Sim;
use sim::bank::SLOTS;

use crate::bag_panel::{self, PAD, cell, count, name_line, slot};
use crate::drag::{Drag, Place};
use crate::hud::{ItemPictures, backdrop};

/// Sizes in logical pixels.
const MARGIN: f64 = 15.0;
const HEADER: f64 = 60.0;
const BUTTON: Vec2 = Vec2::new(300.0, 55.0);
/// Eight across, six down.
const COLUMNS: usize = 8;
/// The layer the panel floats on: over the world, with the others.
const LAYER: usize = 1;

/// What the button says that puts the whole bag in.
pub const PUT_BAG: &str = "Put the bag in";

/// Something a press on the panel asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Take one of what a slot holds.
    Take(usize),
    /// The menu of what can be done with what a slot holds.
    Menu(usize),
    /// Put everything in the bag into the bank.
    PutBag,
}

/// Draw the panel, if the player is at the bank: on the left of the
/// world's area `view`, up the middle. Gives what a press asked for.
/// Anything in it can be carried off, and anything carried put down.
pub fn draw(ui: &mut Ui, view: Rect, sim: &Sim, pictures: &ItemPictures, drag: &mut Drag) -> Option<Asked> {
    if !sim.banking() {
        return None;
    }
    // The metrics are a small copy: the closure does not hold the
    // interface while it is drawn into.
    let m = ui.m;
    let px = |v: f64| m.px(v);
    let (pad, header) = (px(PAD), px(HEADER));
    let size = bag_panel::grid_size(ui, SLOTS, COLUMNS) + Vec2::new(0.0, header);
    let top = (view.center().y - size.y * 0.5).max(view.min.y + px(MARGIN));
    let panel = Rect::from_min_size(Vec2::new(view.min.x + px(MARGIN), top), size).round();
    let mut asked = None;
    ui.child(panel, LAYER, |ui| {
        ui.interact(ui.id("bank"), panel, Sense::NONE);
        backdrop(ui, panel);
        let (plain, big) = (ui.text_style(), ui.heading_style());
        let top = Rect::from_min_size(panel.min + Vec2::splat(pad), Vec2::new(panel.width() - pad * 2.0, header - pad));
        ui.text_in_rect("Bank", &big, top, ui.theme.text);
        let button = Vec2::new(px(BUTTON.x), px(BUTTON.y).min(top.height()));
        ui.child(Rect::new(Vec2::new(top.max.x - button.x, top.min.y), Vec2::new(top.max.x, top.min.y + button.y)), LAYER, |ui| {
            if ui.button_sized(PUT_BAG, button).clicked {
                asked = Some(Asked::PutBag);
            }
        });

        let first = panel.min + Vec2::new(pad, pad + header);
        let mut pointed = None;
        for (i, held) in sim.bank.slots().iter().enumerate() {
            let cell = cell(ui, first, i, COLUMNS);
            let r = ui.interact(ui.id("banked").with_index(i), cell, if held.is_some() { Sense::DRAG } else { Sense::NONE });
            let place = Place::Bank(i);
            drag.slot(ui, place, cell, held.map(|stack| stack.item), &r);
            slot(ui, cell, *held, pictures, (r.hovered && held.is_some()) || drag.aimed_at(ui, cell), drag.lifted_from(place));
            let Some(stack) = held else { continue };
            if r.clicked && !drag.lifted_from(place) {
                asked = Some(Asked::Take(i));
            }
            if r.hovered {
                pointed = Some(*stack);
                if ui.state.right_pressed {
                    asked = Some(Asked::Menu(i));
                }
            }
            // In the bank everything stacks, so everything is counted.
            count(ui, cell, stack.count);
        }
        let line = name_line(ui, panel);
        if let Some(stack) = pointed {
            ui.text_in_rect(stack.item.label(), &plain, line, ui.theme.text);
        }
        ui.text_right(&format!("{} / {SLOTS}", sim.bank.kinds()), &plain, line, ui.theme.text_dim);
    });
    asked
}
