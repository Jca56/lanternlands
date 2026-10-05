//! The skills' panel: a tile for every skill, three to a row, each with
//! its picture, its level and a bar of how far it is to the next. Under
//! them is more about the skill the pointer is on, or the total of them
//! all when it is on none.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_ui::{ImageHandle, Sense, Ui};
use sim::skills::{LEVEL_CAP, xp_for_level};
use sim::{Sim, Skill};

use crate::hud::backdrop;

/// Sizes in logical pixels.
const TILE: Vec2 = Vec2::new(170.0, 105.0);
const GAP: f64 = 5.0;
const PAD: f64 = 10.0;
const ICON: f64 = 70.0;
const BAR: f64 = 12.0;
const COLUMNS: usize = 3;
/// How much bigger than a heading a tile's level is written.
const LEVEL_SCALE: f32 = 1.3;

/// The skills' pictures, uploaded for the interface to draw.
#[derive(Default)]
pub struct SkillPictures(pub [Option<ImageHandle>; Skill::ALL.len()]);

impl SkillPictures {
    fn of(&self, skill: Skill) -> Option<ImageHandle> {
        self.0[Skill::ALL.iter().position(|s| *s == skill)?]
    }
}

/// A number with its thousands marked off: 135,436.
pub fn grouped(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// What is written under the tiles: a heading and what stands at its
/// right end, and two lines under them. For a skill: its name and
/// level, the experience it has, and how much more the next level
/// takes. For none: the total of every level, and all the experience
/// there is.
pub fn details(sim: &Sim, skill: Option<Skill>) -> [String; 4] {
    let Some(skill) = skill else {
        let all: u32 = Skill::ALL.iter().map(|s| sim.skills.xp(*s)).sum();
        return ["Total level".to_owned(), sim.skills.total_level().to_string(), format!("{} experience in all", grouped(all)), String::new()];
    };
    let (level, xp) = (sim.skills.level(skill), sim.skills.xp(skill));
    let next = if level >= LEVEL_CAP { "As high as it goes".to_owned() } else { format!("{} more to level {}", grouped(xp_for_level(level + 1).saturating_sub(xp)), level + 1) };
    [skill.label().to_owned(), format!("Level {level}"), format!("{} experience", grouped(xp)), next]
}

/// How big the panel is.
pub fn size(ui: &Ui) -> Vec2 {
    let px = |v: f64| ui.m.px(v);
    let (columns, rows) = (COLUMNS as f64, Skill::ALL.len().div_ceil(COLUMNS) as f64);
    let written = ui.heading_style().line_height() as f64 + ui.text_style().line_height() as f64 * 2.0;
    Vec2::new(px(TILE.x) * columns + px(GAP) * (columns - 1.0) + px(PAD) * 2.0, (px(TILE.y) + px(GAP)) * rows - px(GAP) + px(PAD) * 3.0 + written)
}

/// Draw the panel in `rect`.
pub fn draw(ui: &mut Ui, rect: Rect, sim: &Sim, pictures: &SkillPictures) {
    backdrop(ui, rect);
    let px = |v: f64| ui.m.px(v);
    let (pad, gap, tile, icon, bar) = (px(PAD), px(GAP), Vec2::new(px(TILE.x), px(TILE.y)), px(ICON), px(BAR));
    let (plain, big) = (ui.text_style(), ui.heading_style());
    let mut numeral = ui.heading_style();
    numeral.size *= LEVEL_SCALE;
    let mut pointed = None;
    for (i, skill) in Skill::ALL.into_iter().enumerate() {
        let at = rect.min + Vec2::new(pad + (i % COLUMNS) as f64 * (tile.x + gap), pad + (i / COLUMNS) as f64 * (tile.y + gap));
        let cell = Rect::from_min_size(at, tile);
        let r = ui.interact(ui.id("skill").with(skill.name()), cell, Sense::NONE);
        ui.recessed(cell, ui.theme.field);
        if r.hovered {
            pointed = Some(skill);
            ui.draw.stroke_rect(cell, ui.m.border * 2.0, ui.m.radius, ui.theme.accent);
        }
        // Along the bottom, how far to the next level; over it, the
        // picture on the left and the level beside it.
        let inside = cell.shrink(gap * 1.5);
        let track = Rect::new(Vec2::new(inside.min.x, inside.max.y - bar), inside.max).round();
        let over = Rect::new(inside.min, Vec2::new(inside.max.x, track.min.y - gap));
        if let Some(image) = pictures.of(skill) {
            let side = icon.min(over.height());
            ui.draw.image(Rect::from_center_size(Vec2::new(over.min.x + side * 0.5, over.center().y), Vec2::new(side, side)).round(), image, 0.0, Color::WHITE);
        }
        let level = sim.skills.level(skill).to_string();
        ui.text_centered(&level, &numeral, Rect::new(Vec2::new(over.min.x + icon, over.min.y), over.max), ui.theme.text);
        let (done, _) = track.split_x(track.min.x + (track.width() * sim.skills.progress(skill).clamp(0.0, 1.0)).round());
        ui.draw.rounded_rect(track, bar * 0.5, ui.theme.text.with_alpha(0.28));
        if done.width() >= 1.0 {
            ui.draw.rounded_rect(done, bar * 0.5, ui.theme.accent);
        }
    }

    let [name, level, has, wants] = details(sim, pointed);
    let rows = Skill::ALL.len().div_ceil(COLUMNS) as f64;
    let (head, line) = (big.line_height() as f64, plain.line_height() as f64);
    let top = rect.min.y + pad * 2.0 + (tile.y + gap) * rows - gap;
    let row = |y: f64, tall: f64| Rect::new(Vec2::new(rect.min.x + pad, y), Vec2::new(rect.max.x - pad, y + tall));
    ui.text_in_rect(&name, &big, row(top, head), ui.theme.text);
    ui.text_right(&level, &big, row(top, head), ui.theme.text);
    ui.text_in_rect(&has, &plain, row(top + head, line), ui.theme.text_dim);
    ui.text_in_rect(&wants, &plain, row(top + head + line, line), ui.theme.text_dim);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_have_their_thousands_marked_off() {
        assert_eq!([grouped(0), grouped(7), grouped(999), grouped(1000), grouped(28_450), grouped(135_436), grouped(4_000_000_000)], ["0", "7", "999", "1,000", "28,450", "135,436", "4,000,000,000"]);
    }

    #[test]
    fn a_skill_pointed_at_tells_its_level_its_experience_and_what_the_next_takes() {
        let mut sim = Sim::new(sim::World::new());
        // Level 5 takes 336; 400 is some way into it.
        sim.skills.set_xp(Skill::Mining, 400);
        let level = sim.skills.level(Skill::Mining);
        let left = xp_for_level(level + 1) - 400;
        assert_eq!(details(&sim, Some(Skill::Mining)), ["Mining".to_owned(), format!("Level {level}"), "400 experience".to_owned(), format!("{} more to level {}", grouped(left), level + 1)]);
        // At the top there is no next.
        sim.skills.set_xp(Skill::Melee, xp_for_level(LEVEL_CAP));
        assert_eq!(details(&sim, Some(Skill::Melee))[1..], ["Level 50".to_owned(), "135,436 experience".to_owned(), "As high as it goes".to_owned()]);
        // A skill never touched is level 1, with the whole of level 2 to go.
        assert_eq!(details(&sim, Some(Skill::Magic))[1..], ["Level 1".to_owned(), "0 experience".to_owned(), "28 more to level 2".to_owned()]);
    }

    #[test]
    fn with_nothing_pointed_at_it_is_the_total() {
        let mut sim = Sim::new(sim::World::new());
        assert_eq!(details(&sim, None), ["Total level".to_owned(), "9".to_owned(), "0 experience in all".to_owned(), String::new()]);
        sim.skills.set_xp(Skill::Mining, 400);
        sim.skills.set_xp(Skill::Melee, 1000);
        let total = sim.skills.total_level();
        assert_eq!(details(&sim, None)[1..3], [total.to_string(), "1,400 experience in all".to_owned()]);
    }
}
