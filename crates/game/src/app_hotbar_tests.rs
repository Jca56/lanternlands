//! The hotbar and the abilities on it, driven with no window.

use sim::Ability;

use super::fights::{body_of, until, with_goblin};
use super::frame::{body, count, snap};
use super::play::{pass, playing, pose_of};
use super::*;
use crate::hotbar;

const PLATE: [u8; 3] = [0xc9, 0xb6, 0x8a];

/// Where a button of the hotbar is on screen.
fn button(h: &Harness, shell: &Shell<Game>, slot: usize) -> Rect {
    h.rect_of(body(shell, View::World).with("ability").with_index(slot)).expect("a hotbar button")
}

/// The game with a goblin beside the player, the two already fighting
/// and neither able to die.
fn fighting(name: &str) -> (Harness, Shell<Game>, Game) {
    let (mut h, mut shell, mut game) = with_goblin(name, 3);
    until(&mut h, &mut shell, &mut game, 30.0, |game| matches!(game.sim.player.activity(), Some(sim::Activity::Fighting { .. })));
    game.sim.monsters[0].health = 12;
    (h, shell, game)
}

#[test]
fn the_bar_is_six_big_buttons_along_the_bottom_clear_of_the_others() {
    let (h, shell, game) = playing("hotbar_place");
    let buttons: Vec<Rect> = (0..hotbar::SLOTS).map(|i| button(&h, &shell, i)).collect();
    for (i, b) in buttons.iter().enumerate() {
        assert!(b.width() >= 90.0 && b.height() >= 90.0, "big: {b:?}");
        assert!(game.view.contains(b.min) && game.view.contains(b.max));
        assert!(i == 0 || b.min.x > buttons[i - 1].max.x, "side by side, left to right");
        assert_eq!(b.min.y, buttons[0].min.y);
    }
    let middle = (buttons[0].min.x + buttons[5].max.x) * 0.5;
    assert!((middle - game.view.center().x).abs() <= 1.0, "in the middle");
    assert!(game.view.max.y - buttons[0].max.y < 40.0, "along the bottom");
    let bag = h.rect_of(body(&shell, View::World).with("Bag")).expect("a Bag button");
    let skills = h.rect_of(body(&shell, View::World).with("Skills")).expect("a Skills button");
    assert!(buttons[5].max.x < skills.min.x && buttons[5].max.x < bag.min.x, "clear of Skills and Bag");
}

#[test]
fn a_number_key_uses_the_ability_on_its_button_at_once() {
    let (mut h, mut shell, mut game) = playing("hotbar_key");
    assert!(!game.sim.acting());
    key(&mut h, &mut shell, &mut game, Key::Char('4'), Modifiers::NONE);
    turn(&mut h, &mut shell, &mut game);
    assert!(game.sim.guard_left() > 0, "the guard is up without waiting for a tick to come round");
    assert!(game.sim.cooldown_left(Ability::Guard) > 0);
    assert!(game.hud.floating().contains(&"guard"));
    // Keys with nothing on their button do nothing.
    for empty in ['5', '6'] {
        key(&mut h, &mut shell, &mut game, Key::Char(empty), Modifiers::NONE);
    }
    assert!(game.hud.said().is_empty());
}

#[test]
fn a_press_on_a_button_uses_it_and_does_not_walk_the_player_anywhere() {
    let (mut h, mut shell, mut game) = playing("hotbar_press");
    let stood = game.sim.player.tile;
    let at = button(&h, &shell, 3).center();
    click(&mut h, &mut shell, &mut game, at);
    pass(&mut h, &mut shell, &mut game, 0.1);
    assert!(game.sim.guard_left() > 0);
    assert_eq!((game.sim.player.goal(), game.sim.player.tile), (None, stood));
    // Nor does a press on the bar between two buttons, or on an empty one.
    let (a, b) = (button(&h, &shell, 0), button(&h, &shell, 1));
    for at in [Vec2::new((a.max.x + b.min.x) * 0.5, a.center().y), button(&h, &shell, 5).center()] {
        click(&mut h, &mut shell, &mut game, at);
        pass(&mut h, &mut shell, &mut game, 0.1);
        assert_eq!((game.sim.player.goal(), game.sim.player.tile), (None, stood));
    }
}

#[test]
fn aimed_at_nobody_an_ability_says_so() {
    let (mut h, mut shell, mut game) = playing("hotbar_nobody");
    key(&mut h, &mut shell, &mut game, Key::Char('1'), Modifiers::NONE);
    turn(&mut h, &mut shell, &mut game);
    assert_eq!(game.hud.said(), vec!["No target"]);
    assert_eq!(game.sim.cooldown_left(Ability::HeavyStrike), 0, "and is not spent");
}

#[test]
fn the_keys_do_nothing_in_the_editor_or_on_the_title_screen() {
    let (mut h, mut shell, mut game) = playing("hotbar_elsewhere");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('4'), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    pass(&mut h, &mut shell, &mut game, 0.5);
    assert!(game.sim.guard_left() == 0 && !game.sim.queued(Ability::Guard) && game.sim.cooldown_left(Ability::Guard) == 0);
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('4'), Modifiers::NONE);
    assert!(!game.sim.queued(Ability::Guard) && game.sim.cooldown_left(Ability::Guard) == 0);
}

#[test]
fn a_heavy_strike_lands_on_the_next_tick_with_a_strike_drawn_and_then_cools() {
    let (mut h, mut shell, mut game) = fighting("hotbar_strike");
    let before = game.sim.monsters[0].health;
    key(&mut h, &mut shell, &mut game, Key::Char('1'), Modifiers::NONE);
    assert!(game.sim.queued(Ability::HeavyStrike));
    let mut struck = false;
    for _ in 0..10 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        if !game.sim.queued(Ability::HeavyStrike) {
            assert_eq!(pose_of(&game), Some(Pose::WorkB), "the strike is drawn as it lands");
            struck = true;
            break;
        }
    }
    assert!(struck, "within half a second");
    assert!(game.sim.monsters[0].health <= before - 2, "for two at least");
    // Its seconds run down in real time, and then it can be used again.
    let whole = Ability::HeavyStrike.cooldown() as f64 * sim::TICK_SECONDS;
    let left = hotbar::seconds_left(&game.sim, Ability::HeavyStrike, game.world_phase / sim::TICK_SECONDS);
    assert!(left > whole - 0.5 && left <= whole, "{left}");
    key(&mut h, &mut shell, &mut game, Key::Char('1'), Modifiers::NONE);
    assert!(!game.sim.queued(Ability::HeavyStrike), "not while it cools");
    for _ in 0..(whole / 0.05) as usize + 12 {
        game.sim.monsters[0].health = 12;
        game.sim.player.health = game.sim.max_health();
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
    }
    assert_eq!(game.sim.cooldown_left(Ability::HeavyStrike), 0);
    key(&mut h, &mut shell, &mut game, Key::Char('1'), Modifiers::NONE);
    assert!(game.sim.queued(Ability::HeavyStrike) || game.sim.cooldown_left(Ability::HeavyStrike) > 0);
}

#[test]
fn a_kicked_goblin_is_said_to_be_stunned_and_stands_still() {
    let (mut h, mut shell, mut game) = fighting("hotbar_kick");
    key(&mut h, &mut shell, &mut game, Key::Char('3'), Modifiers::NONE);
    let floated = until(&mut h, &mut shell, &mut game, 1.0, |game| game.sim.cooldown_left(Ability::Kick) > 0);
    assert!(game.sim.cooldown_left(Ability::Kick) > 0);
    pass(&mut h, &mut shell, &mut game, 0.1);
    assert!(floated.contains(&"stunned".to_owned()) || game.hud.floating().contains(&"stunned"), "{floated:?}");
    let (stood, health) = (game.sim.monsters[0].tile, game.sim.player.health);
    // Two seconds of a goblin that would have swung at least once.
    for _ in 0..40 {
        game.sim.monsters[0].health = 12;
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        assert_eq!(game.sim.monsters[0].tile, stood);
    }
    assert!(game.sim.player.health >= health, "it did not strike while stunned");
}

#[test]
fn while_anything_cools_the_game_keeps_drawing() {
    let (mut h, mut shell, mut game) = playing("hotbar_draws");
    game.sim.monsters.clear();
    pass(&mut h, &mut shell, &mut game, 1.0);
    assert_eq!(turn(&mut h, &mut shell, &mut game).wake_after, None, "at rest nothing is drawn");
    key(&mut h, &mut shell, &mut game, Key::Char('4'), Modifiers::NONE);
    pass(&mut h, &mut shell, &mut game, 5.0);
    assert_eq!(turn(&mut h, &mut shell, &mut game).wake_after, Some(0.0), "the button's wait is drawn running down");
    pass(&mut h, &mut shell, &mut game, Ability::Guard.cooldown() as f64 * sim::TICK_SECONDS);
    assert_eq!(game.sim.cooldown_left(Ability::Guard), 0);
    assert_eq!(turn(&mut h, &mut shell, &mut game).wake_after, None, "and at rest again after");
}

#[test]
fn the_bar_is_drawn_with_its_pictures_and_a_cooling_button_goes_dark() {
    let (mut h, mut shell, mut game) = fighting("frame_hotbar");
    key(&mut h, &mut shell, &mut game, Key::Char('2'), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('4'), Modifiers::NONE);
    for _ in 0..30 {
        game.sim.monsters[0].health = 12;
        game.sim.player.health = game.sim.max_health();
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
    }
    assert!(game.sim.cooldown_left(Ability::Flurry) > 0 && game.sim.guard_left() > 0);
    let at = body_of(&game, 0);
    h.move_to(at);
    turn(&mut h, &mut shell, &mut game);
    let Some(image) = snap("frame-hotbar", &mut h, &mut shell, &mut game) else { return };
    let plate = |slot: usize| count(&image, button(&h, &shell, slot), PLATE, 2);
    // Ready buttons stand on their light plates; cooling ones are dark;
    // empty ones have no plate at all.
    assert!(plate(0) > 1500 && plate(2) > 1500, "ready: {} and {}", plate(0), plate(2));
    assert_eq!((plate(1), plate(3)), (0, 0), "cooling");
    assert_eq!((plate(4), plate(5)), (0, 0), "empty");
    // The seconds left are written on a cooling button in white.
    assert!(count(&image, button(&h, &shell, 1), [255, 255, 255], 20) > 40, "its seconds");
    // Every button with an ability shows its key.
    let chip = |slot: usize| {
        let b = button(&h, &shell, slot);
        count(&image, Rect::from_min_size(b.min, Vec2::new(30.0, 30.0)), [255, 255, 255], 30)
    };
    assert!((0..4).all(|slot| chip(slot) > 10), "the keys' numbers");
}
