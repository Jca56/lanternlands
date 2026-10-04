//! What potions show on screen: under the health bar, each one still
//! at work with the seconds it has left; and on a potion in the bag,
//! the wait before another of its kind can be drunk.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::Ui;
use sim::{Item, Potion, Sim, TICK_SECONDS};

use crate::hotbar::PLATE;
use crate::hud::ItemPictures;

/// Sizes in logical pixels.
const ICON: f64 = 45.0;
const GAP: f64 = 5.0;
/// How wide the seconds beside an icon are given.
const SECONDS: f64 = 45.0;

/// Seconds left of `ticks` world ticks, `into` the tick under way.
fn seconds(ticks: u64, into: f64) -> u32 {
    ((ticks as f64 - into.clamp(0.0, 1.0)) * TICK_SECONDS).ceil().max(0.0) as u32
}

/// Draw, in a row from `at`, every potion still at work.
pub fn draw_effects(ui: &mut Ui, at: Vec2, sim: &Sim, pictures: &ItemPictures, into: f64) {
    let (icon, gap, room) = (ui.m.px(ICON), ui.m.px(GAP), ui.m.px(SECONDS));
    let style = ui.text_style();
    let working = Potion::ALL.into_iter().filter(|potion| sim.potion_left(*potion) > 0);
    for (i, potion) in working.enumerate() {
        let cell = Rect::from_min_size(at + Vec2::new(i as f64 * (icon + room + gap * 2.0), 0.0), Vec2::new(icon, icon)).round();
        ui.draw.rounded_rect(cell, ui.m.radius, PLATE);
        if let Some(image) = pictures.of(potion.item()) {
            ui.draw.image(cell.shrink(gap * 0.5), image, 0.0, Color::WHITE);
        }
        let text = seconds(sim.potion_left(potion), into).to_string();
        let beside = Rect::from_min_size(Vec2::new(cell.max.x + gap, cell.min.y), Vec2::new(room, icon));
        ui.text_in_rect(&text, &style, beside.translate(Vec2::new(0.0, ui.m.px(2.0))), Color::rgba(0.0, 0.0, 0.0, 0.8));
        ui.text_in_rect(&text, &style, beside, Color::WHITE);
    }
}

/// Over a potion in the bag whose kind is still waiting: dark, with
/// the seconds left written on it.
pub fn draw_wait(ui: &mut Ui, cell: Rect, sim: &Sim, item: Item, into: f64) {
    let Some(wait) = item.potion().map(|potion| sim.drink_wait(potion)).filter(|wait| *wait > 0) else {
        return;
    };
    let style = ui.heading_style();
    ui.draw.rounded_rect(cell, ui.m.radius, Color::rgba(0.0, 0.0, 0.0, 0.6));
    let text = seconds(wait, into).to_string();
    ui.text_centered(&text, &style, cell.translate(Vec2::new(0.0, ui.m.px(2.0))), Color::rgba(0.0, 0.0, 0.0, 0.85));
    ui.text_centered(&text, &style, cell, Color::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_count_down_through_a_tick_and_never_below_nothing() {
        assert_eq!((seconds(5, 0.0), seconds(5, 0.5), seconds(5, 1.0)), (2, 2, 2));
        assert_eq!((seconds(3, 0.0), seconds(3, 0.6), seconds(1, 0.9), seconds(0, 0.5)), (2, 1, 1, 0));
    }
}
