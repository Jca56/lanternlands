//! The title screen and the characters' saves, driven with no window.

use super::play::{at_a_rock, pass, press_tile, screen_of};
use super::*;
use crate::title::Slot;

fn slot_id(shell: &Shell<Game>, i: usize) -> WidgetId {
    WidgetId::ROOT.with_u64(shell.screen.target(View::World).unwrap() as u64).with("body").with("character").with_index(i)
}

/// Click a character slot on the title screen.
pub(super) fn click_slot(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, i: usize) -> ShellOutput {
    let at = h.rect_of(slot_id(shell, i)).expect("the title screen's slots are laid out").center();
    click(h, shell, game, at)
}

/// Make a new character in a slot through the title screen's dialog.
pub(super) fn create(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, slot: usize, name: &str) {
    click_slot(h, shell, game, slot);
    assert!(shell.popup_open(), "a dialog asks for the name");
    h.type_text(name);
    turn(h, shell, game);
    key(h, shell, game, Key::Enter, Modifiers::NONE);
    turn(h, shell, game);
}

/// Another run of the game on the same folders: what a restart sees.
fn reopened(game: &Game) -> (Harness, Shell<Game>, Game) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut again = Game::open(&root.join("assets"), game.map.clone(), game.saves.dir().to_owned());
    let mut shell = Shell::new(View::World);
    shell.title_bar = false;
    let mut h = Harness::new(WINDOW.x, WINDOW.y);
    turn(&mut h, &mut shell, &mut again);
    (h, shell, again)
}

#[test]
fn the_game_opens_on_the_title_screen_with_three_empty_slots() {
    let (h, shell, game) = opened("title_opens");
    assert_eq!(game.mode, Mode::Title);
    assert_eq!(game.slots, [Slot::Empty, Slot::Empty, Slot::Empty]);
    assert!((0..3).all(|i| h.rect_of(slot_id(&shell, i)).is_some()));
    // None of the game's own interface is up.
    let body = WidgetId::ROOT.with_u64(shell.screen.target(View::World).unwrap() as u64).with("body");
    assert!(h.rect_of(body.with("Bag")).is_none());
}

#[test]
fn the_title_screen_ignores_the_games_keys_and_clicks_walk_nobody() {
    let (mut h, mut shell, mut game) = opened("title_keys");
    for k in [Key::F(2), Key::Char('i'), Key::Char('k'), Key::Escape] {
        key(&mut h, &mut shell, &mut game, k, Modifiers::NONE);
    }
    assert!(!game.editing && !game.hud.bag_open && !game.hud.skills_open);
    assert_eq!(game.mode, Mode::Title);
    click(&mut h, &mut shell, &mut game, Vec2::new(300.0, 300.0));
    pass(&mut h, &mut shell, &mut game, 1.0);
    assert!(!game.sim.player.busy() && game.sim.ticks == 0);
}

#[test]
fn a_new_character_needs_a_name_and_then_plays() {
    let (mut h, mut shell, mut game) = opened("title_new");
    click_slot(&mut h, &mut shell, &mut game, 1);
    assert!(shell.popup_open());
    // Enter with no name does nothing: Start waits for one.
    key(&mut h, &mut shell, &mut game, Key::Enter, Modifiers::NONE);
    assert!(shell.popup_open() && game.mode == Mode::Title);
    h.type_text("  Alva ");
    turn(&mut h, &mut shell, &mut game);
    key(&mut h, &mut shell, &mut game, Key::Enter, Modifiers::NONE);
    turn(&mut h, &mut shell, &mut game);
    assert_eq!(game.mode, Mode::Playing { slot: 1 });
    assert_eq!(game.character.name, "Alva");
    assert!(!shell.popup_open());
    // They are saved from the first moment, in their own slot's file.
    let saved = game.saves.read(1).expect("a save").expect("that reads");
    assert_eq!((saved.character.name.as_str(), saved.skills.total_level()), ("Alva", 9));
    assert!(game.saves.read(0).is_none() && game.saves.read(2).is_none());
}

#[test]
fn cancelling_the_name_starts_nobody() {
    let (mut h, mut shell, mut game) = opened("title_cancel");
    click_slot(&mut h, &mut shell, &mut game, 0);
    h.type_text("Nobody");
    turn(&mut h, &mut shell, &mut game);
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert!(!shell.popup_open());
    assert_eq!(game.mode, Mode::Title);
    assert!(game.saves.read(0).is_none());
}

#[test]
fn escape_goes_back_to_the_title_and_the_slot_shows_who_is_there() {
    let (mut h, mut shell, mut game) = opened("title_escape");
    create(&mut h, &mut shell, &mut game, 0, "Alva");
    pass(&mut h, &mut shell, &mut game, 125.0);
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert_eq!(game.mode, Mode::Title);
    // (A hundred and twenty-five seconds, give or take the last frame.)
    assert!(matches!(&game.slots[0], Slot::Held { name, total_level: 9, played: 124..=125 } if name == "Alva"), "{:?}", game.slots[0]);
    assert_eq!(game.slots[1], Slot::Empty);
}

#[test]
fn what_was_earned_and_where_they_stood_are_there_after_a_restart() {
    let (mut h, mut shell, mut game) = opened("title_restart");
    create(&mut h, &mut shell, &mut game, 2, "Miner");
    // Stand them on open ground with a rock two tiles east, and mine it.
    let open = |x: i32, y: i32| (-3..=3).all(|dy| (-3..=3).all(|dx| game.sim.world.walkable(x + dx, y + dy)));
    let (x, y) = (4..28).flat_map(|y| (4..44).map(move |x| (x, y))).find(|&(x, y)| open(x, y)).unwrap();
    game.sim.player.tile = (x, y);
    game.sim.settle_player();
    game.camera.center = Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
    game.sim.world.place_prop(PropKind::Rock, x + 2, y);
    game.sim.world.save(&game.map).expect("the map is saved");
    turn(&mut h, &mut shell, &mut game);
    press_tile(&mut h, &mut shell, &mut game, (x + 2, y));
    for _ in 0..600 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        if game.sim.is_spent((x + 2, y)) {
            break;
        }
    }
    assert_eq!(game.sim.inventory.count(Item::CopperOre), 3);
    // Not a word about saving: the game was closed where it stood.
    game.exiting(Exit::Quit);

    let (mut h, mut shell, mut again) = reopened(&game);
    assert_eq!(again.mode, Mode::Title);
    assert!(matches!(&again.slots[2], Slot::Held { name, total_level: 10, .. } if name == "Miner"), "{:?}", again.slots[2]);
    click_slot(&mut h, &mut shell, &mut again, 2);
    assert_eq!(again.mode, Mode::Playing { slot: 2 });
    assert_eq!(again.sim.inventory.count(Item::CopperOre), 3);
    assert_eq!(again.sim.skills.level(Skill::Mining), 2);
    assert_eq!(again.sim.player.tile, (x + 1, y), "beside the rock, where they stood");
    assert!(again.sim.is_spent((x + 2, y)), "the rock they emptied is still empty");
    assert_eq!(again.camera.center, Vec2::new(x as f64 + 1.5, y as f64 + 0.5), "the view starts on them");
}

#[test]
fn an_ore_is_saved_the_moment_it_is_got() {
    let (mut h, mut shell, mut game, rock) = at_a_rock("title_earned");
    press_tile(&mut h, &mut shell, &mut game, rock);
    for _ in 0..600 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        if game.sim.inventory.count(Item::CopperOre) > 0 {
            break;
        }
    }
    // Still mining, nothing pressed: the save on disk already has it.
    let saved = game.saves.read(0).expect("a save").expect("that reads");
    assert_eq!(saved.inventory.count(Item::CopperOre), game.sim.inventory.count(Item::CopperOre));
    assert!(saved.inventory.count(Item::CopperOre) > 0);
}

#[test]
fn walking_is_saved_every_so_often_and_standing_still_is_not_saved_at_all() {
    let (mut h, mut shell, mut game) = super::play::playing("title_walk");
    game.autosave();
    let start = game.saves.read(0).unwrap().unwrap();
    let file = game.saves.dir().join("character-1.save");
    let written = || std::fs::metadata(&file).unwrap().modified().unwrap();
    let before = written();
    // Idle for a long while: nothing happened, nothing is written.
    pass(&mut h, &mut shell, &mut game, 30.0);
    assert_eq!(written(), before);
    // Walk back and forth for longer than the autosave waits.
    let (x, y) = game.sim.player.tile;
    for i in 0..8 {
        let target = if i % 2 == 0 { (x + 3, y) } else { (x, y) };
        h.move_to(screen_of(&game, target));
        turn(&mut h, &mut shell, &mut game);
        h.press();
        turn(&mut h, &mut shell, &mut game);
        h.release();
        pass(&mut h, &mut shell, &mut game, 2.0);
    }
    let saved = game.saves.read(0).unwrap().unwrap();
    assert!(saved.steps > start.steps, "the walking reached the save by itself");
    assert!(saved.character.played >= 10, "and so did the time played: {}", saved.character.played);
}

#[test]
fn two_characters_keep_their_own_things() {
    let (mut h, mut shell, mut game, rock) = at_a_rock("title_two");
    press_tile(&mut h, &mut shell, &mut game, rock);
    for _ in 0..600 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        if game.sim.is_spent(rock) {
            break;
        }
    }
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    create(&mut h, &mut shell, &mut game, 1, "Second");
    assert_eq!(game.mode, Mode::Playing { slot: 1 });
    assert_eq!(game.sim.inventory.count(Item::CopperOre), 0, "a new character starts with nothing");
    assert_eq!(game.sim.skills.total_level(), 9);
    assert_eq!(game.sim.node_left(rock), Some(3), "and the rock the other one emptied is whole for them");
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    click_slot(&mut h, &mut shell, &mut game, 0);
    assert_eq!(game.character.name, "Tester");
    assert_eq!(game.sim.inventory.count(Item::CopperOre), 3);
}

#[test]
fn deleting_asks_first_and_cancel_keeps_the_character() {
    let (mut h, mut shell, mut game) = opened("title_delete");
    create(&mut h, &mut shell, &mut game, 0, "Doomed");
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    let delete = WidgetId::ROOT.with_u64(shell.screen.target(View::World).unwrap() as u64).with("body").with_index(0).with("Delete");
    let at = h.rect_of(delete).expect("a Delete button on the held slot").center();

    click(&mut h, &mut shell, &mut game, at);
    assert!(shell.popup_open(), "it asks first");
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert!(game.saves.read(0).is_some(), "cancelled: still there");

    click(&mut h, &mut shell, &mut game, at);
    key(&mut h, &mut shell, &mut game, Key::Enter, Modifiers::NONE);
    turn(&mut h, &mut shell, &mut game);
    assert!(game.saves.read(0).is_none());
    assert_eq!(game.slots[0], Slot::Empty);
}

#[test]
fn a_save_that_cannot_be_read_is_shown_as_such_and_cannot_be_played() {
    let (mut h, mut shell, mut game) = opened("title_broken");
    std::fs::create_dir_all(game.saves.dir()).unwrap();
    std::fs::write(game.saves.dir().join("character-2.save"), b"rubbish").unwrap();
    game.refresh_slots();
    turn(&mut h, &mut shell, &mut game);
    assert!(matches!(game.slots[1], Slot::Broken(_)));
    click_slot(&mut h, &mut shell, &mut game, 1);
    assert_eq!(game.mode, Mode::Title);
    assert!(!shell.popup_open());
    // The file is left alone until the player deletes it themselves.
    assert!(game.saves.dir().join("character-2.save").exists());
}

#[test]
fn quit_on_the_title_screen_quits() {
    let (mut h, mut shell, mut game) = opened("title_quit");
    let quit = WidgetId::ROOT.with_u64(shell.screen.target(View::World).unwrap() as u64).with("body").with("Quit");
    let at = h.rect_of(quit).expect("a Quit button").center();
    assert!(click(&mut h, &mut shell, &mut game, at).quit);
}

#[test]
fn a_tougher_character_continues_at_their_own_full_health() {
    let (mut h, mut shell, mut game) = opened("title_whole");
    create(&mut h, &mut shell, &mut game, 0, "Tank");
    game.sim.skills.set_xp(Skill::Vitality, sim::skills::xp_for_level(8));
    let whole = game.sim.max_health();
    assert!(whole > 20, "more than a new character has");
    game.sim.player.health = whole;
    game.exiting(Exit::Quit);

    // After a restart, and after a trip to the title screen and back.
    let (mut h, mut shell, mut again) = reopened(&game);
    click_slot(&mut h, &mut shell, &mut again, 0);
    assert_eq!((again.sim.player.health, again.sim.max_health()), (whole, whole));
    key(&mut h, &mut shell, &mut again, Key::Escape, Modifiers::NONE);
    click_slot(&mut h, &mut shell, &mut again, 0);
    assert_eq!(again.sim.player.health, whole);
}
