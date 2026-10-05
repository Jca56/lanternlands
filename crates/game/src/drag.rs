//! Carrying things about with the pointer: a press on something in a
//! slot that moves far enough with the button down picks it up, it
//! follows the pointer, and letting go over another slot puts it down
//! there. A press that goes nowhere is still a plain press.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{CursorIcon, Response, Ui};
use sim::{Item, Slot};

use crate::bag_panel::SLOT;
use crate::hud::ItemPictures;

/// How far the pointer goes with the button down before a press is a
/// carry, in logical pixels.
const LIFT: f64 = 8.0;
/// The layer what is carried is drawn on: over every panel.
const LAYER: usize = 3;

/// A slot something can be carried from or to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Bag(usize),
    Bank(usize),
    Worn(Slot),
}

#[derive(Clone, Copy, Debug)]
struct Carried {
    from: Place,
    item: Item,
    /// It has left its slot: the press has become a carry.
    lifted: bool,
}

#[derive(Default)]
pub struct Drag {
    held: Option<Carried>,
    /// The slot under the pointer, of those drawn so far this frame.
    over: Option<Place>,
}

impl Drag {
    /// Tell of a slot drawn this frame: which it is, where, what it
    /// holds, and what the pointer did to it. A press on something
    /// takes hold of it.
    pub fn slot(&mut self, ui: &Ui, place: Place, cell: Rect, held: Option<Item>, r: &Response) {
        if let Some(item) = held.filter(|_| r.pressed) {
            self.held = Some(Carried { from: place, item, lifted: false });
        }
        if cell.contains(ui.state.pointer) {
            self.over = Some(place);
        }
    }

    /// Whether anything is being carried.
    pub fn carrying(&self) -> bool {
        self.held.is_some_and(|carried| carried.lifted)
    }

    /// Whether what a slot holds is being carried off: it is drawn
    /// faint where it was, and a press let go there is not a press.
    pub fn lifted_from(&self, place: Place) -> bool {
        self.held.is_some_and(|carried| carried.lifted && carried.from == place)
    }

    /// Whether something carried is over a slot: it lights up, to say
    /// it would be put down there.
    pub fn aimed_at(&self, ui: &Ui, cell: Rect) -> bool {
        self.carrying() && cell.contains(ui.state.pointer)
    }

    /// The end of the frame, once every slot has been told of: what is
    /// held is lifted when the pointer has gone far enough, drawn under
    /// the pointer while it is carried, and put down when the button
    /// comes up. Gives where from and where to, when something carried
    /// was let go over another slot.
    pub fn finish(&mut self, ui: &mut Ui, pictures: &ItemPictures) -> Option<(Place, Place)> {
        let over = self.over.take();
        let (down, pointer) = (ui.state.down, ui.state.pointer);
        let carried = self.held.as_mut()?;
        carried.lifted |= down && pointer.distance(ui.state.press_pos) >= ui.m.px(LIFT);
        let carried = *carried;
        if !down {
            self.held = None;
            return over.filter(|to| carried.lifted && *to != carried.from).map(|to| (carried.from, to));
        }
        if carried.lifted {
            if let Some(image) = pictures.of(carried.item) {
                let side = ui.m.px(SLOT);
                let scale = (side / image.width.max(1) as f64).min(side / image.height.max(1) as f64);
                let at = Rect::from_center_size(pointer, Vec2::new(image.width as f64, image.height as f64) * scale).round();
                let layer = ui.draw.layer();
                ui.draw.set_layer(LAYER);
                ui.draw.image(at, image, 0.0, Color::WHITE);
                ui.draw.set_layer(layer);
            }
            ui.state.cursor_icon = CursorIcon::Grabbing;
            ui.state.request_redraw_after(0.0);
        }
        None
    }
}
