//! The panel that opens at a furnace or an anvil: what can be made
//! there, what each takes against what is carried, and buttons to make
//! one, five or all of it.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{Sense, Ui};
use sim::{RECIPES, Sim, Skill};

use crate::hotbar::PLATE;
use crate::hud::ItemPictures;

/// Sizes in logical pixels.
const WIDTH: f64 = 600.0;
const ROW: f64 = 80.0;
const GAP: f64 = 5.0;
const PAD: f64 = 10.0;
const MARGIN: f64 = 15.0;
const HEADER: f64 = 50.0;
const ICON: f64 = 70.0;
const SMALL_ICON: f64 = 30.0;
const BUTTON: Vec2 = Vec2::new(65.0, 55.0);
/// How many are made by each button, and what it says. The last makes
/// all there is the stuff for.
const COUNTS: [(u32, &str); 3] = [(1, "1"), (5, "5"), (u32::MAX, "All")];
/// The layer the panel floats on: over the world, with the others.
const LAYER: usize = 1;

/// What there is not enough of, or not the level for.
const LACKING: Color = Color::hex(0xff7a6b);

/// Where the panel is in the world's area `view`, for a station with
/// `rows` things to make: on the left, up the middle.
fn rect(ui: &Ui, view: Rect, rows: usize) -> Rect {
    let px = |v: f64| ui.m.px(v);
    let height = px(PAD) * 2.0 + px(HEADER) + rows as f64 * (px(ROW) + px(GAP));
    let top = (view.center().y - height * 0.5).max(view.min.y + px(MARGIN));
    Rect::from_min_size(Vec2::new(view.min.x + px(MARGIN), top), Vec2::new(px(WIDTH), height)).round()
}

/// Draw the panel for the station the player is at, if they are at
/// one. Gives the recipe (by its place among them) and how many of it
/// a button pressed this frame asks for.
pub fn draw(ui: &mut Ui, view: Rect, sim: &Sim, pictures: &ItemPictures) -> Option<(usize, u32)> {
    let station = sim.station()?;
    let rows: Vec<usize> = (0..RECIPES.len()).filter(|&i| RECIPES[i].station == station).collect();
    let panel = rect(ui, view, rows.len());
    // The metrics are a small copy: the closure does not hold the
    // interface while it is drawn into.
    let m = ui.m;
    let px = |v: f64| m.px(v);
    let (pad, gap, row_h) = (px(PAD), px(GAP), px(ROW));
    let (icon, small, button) = (px(ICON), px(SMALL_ICON), Vec2::new(px(BUTTON.x), px(BUTTON.y)));
    let mut asked = None;
    ui.child(panel, LAYER, |ui| {
        ui.interact(ui.id("making"), panel, Sense::NONE);
        ui.draw.rounded_rect_gradient(panel, ui.m.radius, ui.theme.panel.top, ui.theme.panel.bottom);
        ui.draw.stroke_rect(panel, ui.m.border, ui.m.radius, ui.theme.border_dark);
        let (plain, big) = (ui.text_style(), ui.heading_style());
        let inside = panel.shrink(pad);
        let header = Rect::from_min_size(inside.min, Vec2::new(inside.width(), px(HEADER)));
        ui.text_in_rect(station.label(), &big, header, ui.theme.text);
        ui.text_right(&format!("Smithing {}", sim.skills.level(Skill::Smithing)), &plain, header, ui.theme.text);

        let making = sim.making();
        for (line, &index) in rows.iter().enumerate() {
            let recipe = &RECIPES[index];
            let row = Rect::from_min_size(Vec2::new(inside.min.x, header.max.y + line as f64 * (row_h + gap)), Vec2::new(inside.width(), row_h));
            ui.recessed(row, ui.theme.field);
            let able = sim.can_smith(recipe);
            let dim = if able { 1.0 } else { 0.45 };

            // What it makes, its name, and under that what it takes
            // against what is carried.
            let picture = Rect::from_center_size(Vec2::new(row.min.x + pad + icon * 0.5, row.center().y), Vec2::new(icon, icon)).round();
            // On a light plate, so its dark outline reads.
            ui.draw.rounded_rect(picture, ui.m.radius, PLATE.with_alpha(dim));
            if let Some(image) = pictures.of(recipe.makes) {
                ui.draw.image(picture.shrink(gap), image, 0.0, Color::WHITE.with_alpha(dim));
            }
            let text_x = picture.max.x + pad;
            let (top, bottom) = row.shrink(gap).split_y(row.center().y);
            ui.text_in_rect(recipe.makes.label(), &plain, Rect::new(Vec2::new(text_x, top.min.y), top.max), ui.theme.text.with_alpha(dim));
            let mut x = text_x;
            for &(item, each) in recipe.from {
                let at = Rect::from_center_size(Vec2::new(x + small * 0.5, bottom.center().y), Vec2::new(small, small)).round();
                if let Some(image) = pictures.of(item) {
                    ui.draw.image(at, image, 0.0, Color::WHITE.with_alpha(dim));
                }
                let have = sim.inventory.count(item);
                let text = format!("{have}/{each}");
                let width = ui.measure(&text, &plain);
                let color = if have >= each { ui.theme.text } else { LACKING };
                ui.text_in_rect(&text, &plain, Rect::new(Vec2::new(at.max.x + gap, bottom.min.y), bottom.max), color.with_alpha(dim));
                x = at.max.x + gap + width + pad * 1.5;
            }

            // At the right: the level it wants, how many are still to
            // make, or the buttons to make some.
            let right = Rect::new(Vec2::new(row.max.x - pad - button.x * 3.0 - gap * 2.0, row.min.y), row.max - Vec2::new(pad, 0.0));
            if !able {
                ui.text_right(&format!("Smithing {}", recipe.level), &plain, right, LACKING);
                continue;
            }
            if let Some((_, left, _)) = making.filter(|(now, _, _)| std::ptr::eq(*now, recipe)) {
                ui.draw.stroke_rect(row, px(3.0), ui.m.radius, ui.theme.accent);
                ui.text_right(&format!("{left} to go"), &plain, right, ui.theme.text);
                continue;
            }
            let enough = sim.makeable(recipe) > 0;
            for (k, (count, label)) in COUNTS.iter().enumerate() {
                let cell = Rect::from_center_size(Vec2::new(right.min.x + button.x * 0.5 + k as f64 * (button.x + gap), row.center().y), button).round();
                let sense = if enough { Sense::CLICK } else { Sense::NONE };
                let r = ui.interact(ui.id("make").with_index(index).with(label), cell, sense);
                if enough {
                    let face = ui.widget_color(&r);
                    ui.raised(cell, face, r.held);
                    if r.clicked {
                        asked = Some((index, *count));
                    }
                } else {
                    ui.recessed(cell, ui.theme.field);
                }
                ui.text_centered(label, &plain, cell, ui.theme.text.with_alpha(if enough { 1.0 } else { 0.35 }));
            }
        }
    });
    asked
}
