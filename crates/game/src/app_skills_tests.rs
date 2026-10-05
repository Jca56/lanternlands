//! The skills' panel, driven and drawn with no window.

use super::frame::{GRASS, INK, body, count, snap};
use super::play::playing;
use super::*;

/// The red of the heart on the Vitality tile, and the sealing-wax red
/// the bars are filled with.
const HEART: [u8; 3] = [0xc8, 0x46, 0x3c];
const WAX: [u8; 3] = [0x9c, 0x3d, 0x2a];

fn tile(h: &Harness, shell: &Shell<Game>, skill: Skill) -> Rect {
    h.rect_of(body(shell, View::World).with("skill").with(skill.name())).unwrap_or_else(|| panic!("no tile for {}", skill.label()))
}

/// The game with a miner some way into a level, and the skills open.
fn opened(name: &str) -> (Harness, Shell<Game>, Game) {
    let (mut h, mut shell, mut game) = playing(name);
    game.sim.monsters.clear();
    game.sim.skills.set_xp(Skill::Mining, 400);
    key(&mut h, &mut shell, &mut game, Key::Char('k'), Modifiers::NONE);
    assert!(game.hud.skills_open);
    (h, shell, game)
}

#[test]
fn every_skill_has_a_big_tile_three_to_a_row() {
    let (h, shell, _) = opened("skills_tiles");
    let tiles: Vec<Rect> = Skill::ALL.into_iter().map(|skill| tile(&h, &shell, skill)).collect();
    for (skill, tile) in Skill::ALL.into_iter().zip(&tiles) {
        assert!(tile.width() >= 160.0 && tile.height() >= 100.0, "{}: {tile:?}", skill.label());
        assert!(tile.max.x <= WINDOW.x && tile.max.y < WINDOW.y - 60.0, "{} is on screen, over the buttons", skill.label());
    }
    // Three to a row, the rows one under another, and none over another.
    assert!(tiles[0].min.y == tiles[2].min.y && tiles[0].max.x < tiles[1].min.x && tiles[1].max.x < tiles[2].min.x);
    assert!(tiles[3].min.y > tiles[0].max.y && tiles[3].min.x == tiles[0].min.x && tiles[8].min.y > tiles[5].max.y);
    // In the order the skills are told in: fighting, then gathering, then making.
    assert_eq!(Skill::ALL[5], Skill::Mining);
    assert_eq!(tile(&h, &shell, Skill::Mining), tiles[5]);
}

#[test]
fn the_tiles_show_pictures_levels_and_bars_and_pointing_at_one_tells_more() {
    let (mut h, mut shell, mut game) = opened("frame_skills");
    let Some(image) = snap("frame-skills", &mut h, &mut shell, &mut game) else { return };
    let (mining, melee, vitality) = (tile(&h, &shell, Skill::Mining), tile(&h, &shell, Skill::Melee), tile(&h, &shell, Skill::Vitality));
    // The panel covers the world; each tile has its own picture.
    assert_eq!(count(&image, mining.union(&melee).union(&vitality), GRASS, 1), 0, "no grass shows through the tiles");
    assert!(count(&image, vitality, HEART, 6) > 800, "the heart: {}", count(&image, vitality, HEART, 6));
    assert_eq!(count(&image, mining, HEART, 6), 0);
    // A skill some way to its next level has that much of its bar
    // filled; one not begun has none.
    assert!(count(&image, mining, WAX, 4) > 150, "mining's bar: {}", count(&image, mining, WAX, 4));
    assert_eq!(count(&image, melee, WAX, 4), 0, "melee has not begun");
    // The level is written big, in ink.
    let numeral = Rect::new(Vec2::new(melee.center().x, melee.min.y), melee.max - Vec2::new(0.0, 30.0));
    assert!(count(&image, numeral, INK, 12) > 150, "the level's numeral: {}", count(&image, numeral, INK, 12));

    // Under the tiles: the total, and nothing on the last line until a
    // skill is pointed at.
    let (first, last) = (tile(&h, &shell, Skill::Herbalism), tile(&h, &shell, Skill::Alchemy));
    // (Clear of the panel's own edge, which is nearly the same brown.)
    let last_line = Rect::new(Vec2::new(first.min.x, last.max.y + 90.0), Vec2::new(last.max.x, last.max.y + 108.0));
    const FADED: [u8; 3] = [0x6b, 0x56, 0x38];
    assert_eq!(count(&image, last_line, FADED, 40), 0, "the last line is empty");
    h.move_to(mining.center());
    turn(&mut h, &mut shell, &mut game);
    turn(&mut h, &mut shell, &mut game);
    let Some(pointed) = snap("frame-skills-pointed", &mut h, &mut shell, &mut game) else { return };
    assert!(count(&pointed, last_line, FADED, 40) > 200, "what the next level takes: {}", count(&pointed, last_line, FADED, 40));
}
