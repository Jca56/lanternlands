//! Talking with people: the name over each one's head, and the box
//! what they say is written in, a line at a time.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{Sense, Ui};
use sim::Sim;

use crate::hotbar;

/// Sizes in logical pixels.
const WIDTH: f64 = 900.0;
const PAD: f64 = 20.0;
const GAP: f64 = 10.0;
const MARGIN: f64 = 15.0;
const HEADER: f64 = 50.0;
const BUTTON: Vec2 = Vec2::new(190.0, 60.0);
/// The fewest lines of writing the box has room for.
const LINES: f64 = 3.0;
/// How tall a person is, in tiles from their feet: their name is
/// written just over that.
const HEAD: f64 = 1.45;
/// The layer the box floats on: over the panels, under the shell's
/// own popups.
const LAYER: usize = 2;

/// What the button says that hears the next line, and what it says
/// after the last.
pub const NEXT: &str = "Next";
pub const DONE: &str = "Goodbye";

/// Each person's name over their head, on a dark plate so it reads
/// over anything. `feet` is where each one's feet are drawn, in tiles.
pub fn draw_names(ui: &mut Ui, sim: &Sim, feet: &[Vec2], to_screen: &dyn Fn(Vec2) -> Vec2) {
    let style = ui.text_style();
    let (line, gap) = (style.line_height() as f64, ui.m.px(GAP));
    for (person, &feet) in sim.people.iter().zip(feet) {
        let name = person.kind.label();
        let size = Vec2::new(ui.measure(name, &style) + gap * 2.0, line + gap * 0.5);
        // The same few pixels clear of their head, however far the
        // view is zoomed.
        let middle = to_screen(feet - Vec2::new(0.0, HEAD)) - Vec2::new(0.0, size.y * 0.5 + gap * 0.5);
        let plate = Rect::from_center_size(middle, size).round();
        ui.draw.rounded_rect(plate, ui.m.radius, Color::rgba(0.0, 0.0, 0.0, 0.55));
        ui.text_centered(name, &style, plate, Color::WHITE);
    }
}

/// Draw what is being said to the player, if someone is talking to
/// them: who says it, the line, how far through they are, and a big
/// button for the next. The box stands on the hotbar, in the world's
/// area `view`. Gives whether a press asked to hear on.
pub fn draw(ui: &mut Ui, view: Rect, sim: &Sim) -> bool {
    let Some(saying) = sim.saying() else {
        return false;
    };
    // The metrics are a small copy: the closure does not hold the
    // interface while it is drawn into.
    let m = ui.m;
    let px = |v: f64| m.px(v);
    let (plain, big) = (ui.text_style(), ui.heading_style());
    let (pad, gap, button) = (px(PAD), px(GAP), Vec2::new(px(BUTTON.x), px(BUTTON.y)));
    let width = px(WIDTH).min(view.width() - px(MARGIN) * 2.0);
    // Room for the longest thing they have to say, so the box keeps its
    // size from one line to the next.
    let tallest = saying.kind.lines().iter().map(|line| ui.text.measure_wrapped(line, &plain, (width - pad * 2.0) as f32).height as f64).fold(0.0, f64::max);
    let writing = tallest.max(plain.line_height() as f64 * LINES);
    let height = pad * 2.0 + px(HEADER) + writing + gap + button.y;
    let bottom = hotbar::rect(ui, view).min.y - gap;
    let panel = Rect::from_min_size(Vec2::new(view.center().x - width * 0.5, bottom - height), Vec2::new(width, height)).round();
    let mut next = false;
    ui.child(panel, LAYER, |ui| {
        ui.draw.rounded_rect_gradient(panel, ui.m.radius, ui.theme.panel.top, ui.theme.panel.bottom);
        ui.draw.stroke_rect(panel, ui.m.border, ui.m.radius, ui.theme.border_dark);
        let inside = panel.shrink(pad);
        let header = Rect::from_min_size(inside.min, Vec2::new(inside.width(), px(HEADER)));
        ui.text_in_rect(saying.kind.label(), &big, header, ui.theme.text);
        ui.text_right(saying.kind.calling(), &plain, header, ui.theme.text_dim);
        ui.text_at(saying.text, &plain, Vec2::new(inside.min.x, header.max.y), inside.width(), ui.theme.text);

        let foot = Rect::new(Vec2::new(inside.min.x, inside.max.y - button.y), inside.max);
        ui.text_in_rect(&format!("{} / {}", saying.line, saying.of), &plain, foot, ui.theme.text_dim);
        let label = if saying.line < saying.of { NEXT } else { DONE };
        ui.child(Rect::new(Vec2::new(foot.max.x - button.x, foot.min.y), foot.max), LAYER, |ui| {
            next = ui.button_sized(label, button).clicked;
        });
        // A press anywhere else on the box does the same. Asked for
        // after the button, so the button has the first say.
        next |= ui.interact(ui.id("talk"), panel, Sense::CLICK).clicked;
    });
    next
}
