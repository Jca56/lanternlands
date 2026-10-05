//! The ring on the ground under whatever the player is set on: gold
//! under a thing they use, blue under someone they talk with.

use sim::{Grave, Inventory, PersonKind};

use super::fights::until;
use super::frame::{count, snap};
use super::people::{body_of, hail, with_person};
use super::play::{pass, playing, press_tile, screen_of};
use super::*;
use crate::app::view::{RING_TALK, RING_USE};
use crate::scene::Placed;

/// The rings' colours as they come out on screen, and the walk ring's
/// gold as it comes out over grass.
const GOLD: [u8; 3] = [0xc1, 0x9b, 0x4a];
const BLUE: [u8; 3] = [0x78, 0xae, 0xf0];
const WALK_GOLD: [u8; 3] = [0xba, 0x9c, 0x4a];

/// Every ring in the scene, of either width.
fn rings(game: &Game) -> Vec<Placed> {
    let (ring, wide) = (game.look.target_ring, game.look.target_ring_wide);
    assert!(ring.is_some() && wide.is_some(), "both rings are in the assets");
    game.extras().into_iter().filter(|p| Some(p.sprite) == ring || Some(p.sprite) == wide).collect()
}

/// Whether the one ring in the scene is the wide one.
fn wide(game: &Game) -> bool {
    rings(game).first().map(|p| p.sprite) == game.look.target_ring_wide
}

/// Whether the one ring in the scene lies under whatever stands on
/// `ground`, in `tint`: on the same spot, and drawn before it.
fn ringed(game: &Game, ground: Vec2, tint: [f32; 4]) -> bool {
    let rings = rings(game);
    rings.len() == 1 && rings[0].tint == tint && (rings[0].ground - ground).length() < 0.001 && rings[0].ground.y < ground.y
}

fn middle(tile: (i32, i32)) -> Vec2 {
    Vec2::new(tile.0 as f64 + 0.5, tile.1 as f64 + 0.5)
}

/// The game with the player on open grass and a prop `east` tiles east.
fn with_prop(name: &str, kind: PropKind, east: i32) -> (Harness, Shell<Game>, Game, (i32, i32)) {
    let (mut h, mut shell, mut game) = playing(name);
    game.sim.monsters.clear();
    let (x, y) = game.sim.player.tile;
    let at = (x + east, y);
    game.sim.world.remove_prop(at.0, at.1);
    assert!(game.sim.world.place_prop(kind, at.0, at.1));
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game, at)
}

#[test]
fn a_rock_clicked_is_ringed_in_gold_from_the_click_until_it_is_spent() {
    let (mut h, mut shell, mut game, rock) = with_prop("ring_rock", PropKind::Rock, 4);
    assert!(rings(&game).is_empty(), "nothing is ringed before a click");
    press_tile(&mut h, &mut shell, &mut game, rock);
    assert!(game.sim.player.goal().is_some(), "still on the way");
    // On the way to it and working it, all the way to its last ore.
    for _ in 0..2400 {
        if game.sim.is_spent(rock) {
            break;
        }
        assert!(ringed(&game, middle(rock), RING_USE), "{:?}", rings(&game));
        assert!(wide(&game), "a rock is wider than a tile: its ring shows all round it");
        pass(&mut h, &mut shell, &mut game, 0.05);
    }
    assert!(game.sim.is_spent(rock));
    assert!(rings(&game).is_empty(), "a spent rock is let go of");
}

#[test]
fn walking_off_lets_go_of_what_was_ringed() {
    let (mut h, mut shell, mut game, rock) = with_prop("ring_leave", PropKind::Rock, 4);
    press_tile(&mut h, &mut shell, &mut game, rock);
    assert!(ringed(&game, middle(rock), RING_USE));
    let (x, y) = game.sim.player.tile;
    press_tile(&mut h, &mut shell, &mut game, (x, y + 2));
    assert!(rings(&game).is_empty());
    assert_eq!(game.sim.player.goal(), Some((x, y + 2)));
}

#[test]
fn a_herb_and_a_station_are_ringed_the_same_way() {
    let (mut h, mut shell, mut game, herb) = with_prop("ring_herb", PropKind::HerbPurple, 3);
    press_tile(&mut h, &mut shell, &mut game, herb);
    assert!(ringed(&game, middle(herb), RING_USE) && !wide(&game), "a herb is small: the plain ring");
    until(&mut h, &mut shell, &mut game, 20.0, |game| game.sim.is_spent(herb));
    assert!(rings(&game).is_empty());

    // A station keeps its ring for as long as the player stands at it.
    for (n, kind) in [PropKind::Furnace, PropKind::Anvil, PropKind::Cauldron].into_iter().enumerate() {
        let (mut h, mut shell, mut game, at) = with_prop(&format!("ring_station{n}"), kind, 3);
        press_tile(&mut h, &mut shell, &mut game, at);
        assert!(ringed(&game, middle(at), RING_USE) && wide(&game), "{kind:?} on the way");
        until(&mut h, &mut shell, &mut game, 10.0, |game| game.sim.station().is_some());
        pass(&mut h, &mut shell, &mut game, 2.0);
        assert!(game.sim.station().is_some() && ringed(&game, middle(at), RING_USE), "{kind:?} standing at it");
        key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
        assert!(rings(&game).is_empty(), "{kind:?} stepped back from");
    }
}

#[test]
fn the_grave_is_ringed_while_the_player_goes_back_for_it() {
    let (mut h, mut shell, mut game) = playing("ring_grave");
    game.sim.monsters.clear();
    let (x, y) = game.sim.player.tile;
    let mut items = Inventory::default();
    items.add(Item::CopperOre, 2);
    game.sim.grave = Some(Grave { tile: (x + 4, y), items });
    turn(&mut h, &mut shell, &mut game);
    press_tile(&mut h, &mut shell, &mut game, (x + 4, y));
    assert!(ringed(&game, middle((x + 4, y)), RING_USE) && !wide(&game));
    until(&mut h, &mut shell, &mut game, 20.0, |game| game.sim.grave.is_none());
    assert!(game.sim.grave.is_none() && rings(&game).is_empty());
}

#[test]
fn whoever_the_player_is_coming_to_or_talking_with_is_ringed_in_blue() {
    let (mut h, mut shell, mut game) = with_person("ring_person", PersonKind::Herbalist, 4);
    assert!(rings(&game).is_empty());
    let at = body_of(&game, 0);
    click(&mut h, &mut shell, &mut game, at);
    assert!(ringed(&game, game.person_grounds()[0], RING_TALK) && !wide(&game), "on the way to them");
    hail(&mut h, &mut shell, &mut game, 0);
    assert!(ringed(&game, game.person_grounds()[0], RING_TALK), "and while they talk");
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert!(rings(&game).is_empty());
}

/// The square of ground a ring lies on, around `ground`: room for the
/// wide one.
fn ground_rect(game: &Game, ground: Vec2) -> Rect {
    let px = game.camera.tile_px(game.scale) * 1.5;
    Rect::from_center_size(game.camera.to_screen(ground, game.view, game.scale), Vec2::new(px, px))
}

#[test]
fn the_rings_are_drawn_gold_under_a_rock_and_blue_under_a_person() {
    let (mut h, mut shell, mut game, rock) = with_prop("frame_ring_use", PropKind::Rock, 4);
    let Some(plain) = snap("frame-ring-use-none", &mut h, &mut shell, &mut game) else { return };
    let under_rock = ground_rect(&game, middle(rock));
    assert_eq!(count(&plain, under_rock, GOLD, 3), 0, "no gold there before the click");
    press_tile(&mut h, &mut shell, &mut game, rock);
    let goal = game.sim.player.goal().expect("on the way to it");
    let Some(image) = snap("frame-ring-use", &mut h, &mut shell, &mut game) else { return };
    assert!(count(&image, under_rock, GOLD, 3) > 200, "the rock's ring: {}", count(&image, under_rock, GOLD, 3));
    // The tile they are walking to has no ring of its own: under where
    // their feet will be, clear of the rock's ring, the grass is bare.
    let at = screen_of(&game, goal);
    let below = Rect::new(at + Vec2::new(-40.0, 8.0), at + Vec2::new(15.0, 40.0));
    assert_eq!(count(&image, below, WALK_GOLD, 16), 0, "no walk ring beside a ringed rock");

    let (mut h, mut shell, mut game) = with_person("frame_ring_talk", PersonKind::Smith, 4);
    let at = body_of(&game, 0);
    click(&mut h, &mut shell, &mut game, at);
    let Some(image) = snap("frame-ring-talk", &mut h, &mut shell, &mut game) else { return };
    let under_them = ground_rect(&game, game.person_grounds()[0]);
    assert!(count(&image, under_them, BLUE, 3) > 200, "the smith's ring: {}", count(&image, under_them, BLUE, 3));
    assert_eq!(count(&image, under_them, GOLD, 3), 0);
}
