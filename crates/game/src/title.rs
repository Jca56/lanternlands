//! The title screen: the painting, the game's name, and the characters
//! to pick from.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{ImageHandle, Sense, Ui};

use crate::saves::SLOTS;

/// What the game is called, for now.
pub const GAME_NAME: &str = "Explore the World";
/// The longest a character's name may be, in letters.
pub const NAME_MAX: usize = 16;

/// Sizes in logical pixels.
const COLUMN: f64 = 640.0;
const ROW: f64 = 110.0;
const DELETE_WIDTH: f64 = 130.0;
const QUIT_HEIGHT: f64 = 55.0;
const MARGIN: f64 = 60.0;
const PAD: f64 = 20.0;
const GAP: f64 = 10.0;
/// How much bigger than a heading the game's name is written.
const NAME_SCALE: f32 = 2.0;

/// What a character slot holds, as the title screen shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slot {
    Empty,
    Held { name: String, total_level: u32, played: u64 },
    /// The slot's file is there and cannot be read: why.
    Broken(String),
}

/// What was picked on the title screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chosen {
    Continue(usize),
    New(usize),
    Delete(usize),
    Quit,
}

/// A name as it will be kept: trimmed, and `None` if that leaves
/// nothing, or too much, or something that is not a letter to read.
pub fn valid_name(typed: &str) -> Option<String> {
    let name = typed.trim();
    let fits = (1..=NAME_MAX).contains(&name.chars().count());
    (fits && !name.chars().any(char::is_control)).then(|| name.to_owned())
}

/// Time played, in words: "12 min", "3 h 05 min".
pub fn played_text(seconds: u64) -> String {
    let minutes = seconds / 60;
    if minutes < 60 { format!("{minutes} min") } else { format!("{} h {:02} min", minutes / 60, minutes % 60) }
}

/// The part of a picture `image` wide by tall that fills `view` without
/// stretching, as a rect in `0..1` both ways: its middle, cut at the
/// sides or at top and bottom.
pub fn cover(image: Vec2, view: Vec2) -> Rect {
    let (picture, window) = (image.x / image.y, view.x / view.y);
    let size = if window > picture { Vec2::new(1.0, picture / window) } else { Vec2::new(window / picture, 1.0) };
    Rect::from_center_size(Vec2::new(0.5, 0.5), size)
}

/// Draw the title screen over `view`. Returns what was picked, if
/// anything was this frame.
pub fn draw(ui: &mut Ui, view: Rect, painting: Option<ImageHandle>, slots: &[Slot; SLOTS]) -> Option<Chosen> {
    match painting {
        Some(image) => ui.draw.image_uv(view, image, cover(Vec2::new(image.width as f64, image.height as f64), view.size()), 0.0, Color::WHITE),
        // The sky's own colour, if the painting is missing.
        None => ui.draw.rect(view, Color::hex(0xa9c6d2)),
    }
    let m = ui.m;
    let px = |v: f64| m.px(v);
    let (pad, gap, row) = (px(PAD), px(GAP), px(ROW));
    let mut name_style = ui.heading_style();
    name_style.size *= NAME_SCALE;
    let name_height = name_style.line_height() as f64;

    // A dark plate down the right of the screen holds everything.
    let width = px(COLUMN).min(view.width() - pad * 2.0);
    let height = name_height + pad + (row + gap) * SLOTS as f64 + px(QUIT_HEIGHT);
    let right = view.max.x - px(MARGIN).min((view.width() - width) * 0.5);
    let top = (view.center().y - height * 0.5).round();
    let column = Rect::new(Vec2::new(right - width, top), Vec2::new(right, top + height));
    ui.draw.rounded_rect(column.expand(pad), m.radius * 2.0, Color::rgba(0.0, 0.0, 0.0, 0.55));

    let name_rect = Rect::from_min_size(column.min, Vec2::new(width, name_height));
    ui.text_centered(GAME_NAME, &name_style, name_rect, Color::WHITE);

    let mut chosen = None;
    let heading = ui.heading_style();
    let text = ui.text_style();
    for (i, slot) in slots.iter().enumerate() {
        let at = Vec2::new(column.min.x, name_rect.max.y + pad + i as f64 * (row + gap));
        let whole = Rect::from_min_size(at, Vec2::new(width, row));
        // A held or broken slot gives up its right end to a Delete button.
        let deletable = *slot != Slot::Empty;
        let main = if deletable { Rect::new(whole.min, whole.max - Vec2::new(px(DELETE_WIDTH) + gap, 0.0)) } else { whole };
        let id = ui.id("character").with_index(i);
        let mut r = ui.interact(id, main, Sense::CLICK);
        ui.focusable(id, main);
        ui.key_click(id, &mut r);
        ui.button_face(main, &r);
        let inner = main.shrink(pad);
        let (upper, lower) = inner.split_y(inner.center().y);
        match slot {
            Slot::Empty => {
                ui.text_centered("New character", &heading, main, ui.theme.text_dim);
                if r.clicked {
                    chosen = Some(Chosen::New(i));
                }
            }
            Slot::Held { name, total_level, played } => {
                ui.text_in_rect(name, &heading, upper, ui.theme.text);
                ui.text_in_rect(&format!("Total level {total_level}"), &text, lower, ui.theme.text_dim);
                ui.text_right(&played_text(*played), &text, lower, ui.theme.text_dim);
                if r.clicked {
                    chosen = Some(Chosen::Continue(i));
                }
            }
            Slot::Broken(_) => ui.text_in_rect("This save cannot be read", &text, inner, ui.theme.text_dim),
        }
        ui.focus_ring(id, main);
        if deletable {
            let delete = Rect::new(Vec2::new(main.max.x + gap, whole.min.y), whole.max);
            ui.child(delete, ui.layer(), |ui| {
                ui.push_index(i);
                if ui.button_sized("Delete", delete.size()).clicked {
                    chosen = Some(Chosen::Delete(i));
                }
                ui.pop_id();
            });
        }
    }
    let quit = Rect::new(Vec2::new(column.min.x, column.max.y - px(QUIT_HEIGHT)), column.max);
    ui.child(quit, ui.layer(), |ui| {
        if ui.button_sized("Quit", quit.size()).clicked {
            chosen = Some(Chosen::Quit);
        }
    });
    chosen
}

#[cfg(test)]
mod tests {
    use lntrn_ui::WidgetId;
    use lntrn_ui::testing::Harness;

    use super::*;

    const WINDOW: Vec2 = Vec2::new(1600.0, 1000.0);

    fn slots() -> [Slot; SLOTS] {
        [Slot::Held { name: "Alva".to_owned(), total_level: 12, played: 4000 }, Slot::Empty, Slot::Broken("no good".to_owned())]
    }

    /// One frame of the title screen, with whatever input is queued.
    fn frame(h: &mut Harness, slots: &[Slot; SLOTS]) -> Option<Chosen> {
        let mut chosen = None;
        h.frame(|ui| chosen = draw(ui, Rect::new(Vec2::ZERO, WINDOW), None, slots));
        chosen
    }

    fn click(h: &mut Harness, slots: &[Slot; SLOTS], id: WidgetId) -> Option<Chosen> {
        let mut chosen = None;
        h.click_on(id, |ui| chosen = chosen.or(draw(ui, Rect::new(Vec2::ZERO, WINDOW), None, slots)));
        chosen
    }

    fn character(i: usize) -> WidgetId {
        WidgetId::ROOT.with("character").with_index(i)
    }

    #[test]
    fn a_held_slot_continues_an_empty_one_starts_and_nothing_is_picked_untouched() {
        let (mut h, slots) = (Harness::new(WINDOW.x, WINDOW.y), slots());
        assert_eq!(frame(&mut h, &slots), None);
        assert_eq!(click(&mut h, &slots, character(0)), Some(Chosen::Continue(0)));
        assert_eq!(click(&mut h, &slots, character(1)), Some(Chosen::New(1)));
        assert_eq!(click(&mut h, &slots, character(2)), None, "a save that cannot be read cannot be played");
    }

    #[test]
    fn held_and_broken_slots_can_be_deleted_and_empty_ones_cannot() {
        let (mut h, slots) = (Harness::new(WINDOW.x, WINDOW.y), slots());
        frame(&mut h, &slots);
        let delete = |i: usize| WidgetId::ROOT.with_index(i).with("Delete");
        assert!(h.rect_of(delete(1)).is_none(), "nothing to delete in an empty slot");
        assert_eq!(click(&mut h, &slots, delete(0)), Some(Chosen::Delete(0)));
        assert_eq!(click(&mut h, &slots, delete(2)), Some(Chosen::Delete(2)));
        assert_eq!(click(&mut h, &slots, WidgetId::ROOT.with("Quit")), Some(Chosen::Quit));
    }

    #[test]
    fn the_slots_are_big_and_none_overlaps_another_or_the_buttons() {
        let (mut h, slots) = (Harness::new(WINDOW.x, WINDOW.y), slots());
        frame(&mut h, &slots);
        let mut rects: Vec<Rect> = (0..SLOTS).map(|i| h.rect_of(character(i)).expect("a slot")).collect();
        assert!(rects.iter().all(|r| r.height() >= 110.0 && r.width() >= 400.0), "{rects:?}");
        rects.push(h.rect_of(WidgetId::ROOT.with("Quit")).expect("a Quit button"));
        rects.push(h.rect_of(WidgetId::ROOT.with_index(0).with("Delete")).expect("a Delete button"));
        for (i, a) in rects.iter().enumerate() {
            assert!(a.min.x >= 0.0 && a.max.x <= WINDOW.x && a.min.y >= 0.0 && a.max.y <= WINDOW.y, "{a:?} is off the window");
            assert!(rects[i + 1..].iter().all(|b| !a.intersects(b)), "{a:?} overlaps another");
        }
    }

    #[test]
    fn names_are_trimmed_and_must_be_one_to_sixteen_letters() {
        assert_eq!(valid_name("  Alva "), Some("Alva".to_owned()));
        assert_eq!(valid_name("Ælfgifu the Bold"), Some("Ælfgifu the Bold".to_owned()));
        assert_eq!(valid_name(""), None);
        assert_eq!(valid_name("   "), None);
        assert_eq!(valid_name("seventeen letters"), None);
        assert_eq!(valid_name("tab\there"), None);
    }

    #[test]
    fn time_played_reads_in_minutes_then_hours() {
        assert_eq!(played_text(0), "0 min");
        assert_eq!(played_text(59), "0 min");
        assert_eq!(played_text(3599), "59 min");
        assert_eq!(played_text(3600), "1 h 00 min");
        assert_eq!(played_text(3 * 3600 + 5 * 60 + 9), "3 h 05 min");
    }

    #[test]
    fn the_painting_fills_the_view_from_its_middle_without_stretching() {
        let painting = Vec2::new(1920.0, 1080.0);
        // The same shape: all of it.
        assert_eq!(cover(painting, Vec2::new(960.0, 540.0)), Rect::new(Vec2::ZERO, Vec2::ONE));
        // A wider window loses top and bottom; a taller one loses the sides.
        let wide = cover(painting, Vec2::new(2000.0, 500.0));
        assert_eq!((wide.min.x, wide.max.x), (0.0, 1.0));
        assert!((wide.height() - (1920.0 / 1080.0) / 4.0).abs() < 1e-12 && (wide.center().y - 0.5).abs() < 1e-12);
        let tall = cover(painting, Vec2::new(540.0, 1080.0));
        assert_eq!((tall.min.y, tall.max.y), (0.0, 1.0));
        assert!((tall.width() - 0.5 / (1920.0 / 1080.0)).abs() < 1e-12);
    }
}
