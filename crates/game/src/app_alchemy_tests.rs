//! Picking herbs, brewing and drinking, driven with no window.

use sim::{Potion, RECIPES, Station};

use super::fights::until;
use super::frame::{body, count, snap};
use super::play::{pass, playing, press_tile, screen_of};
use super::*;

/// The game with a purple herb two tiles east of the player and a
/// cauldron two tiles west, and nothing about to bother them.
fn garden(name: &str) -> (Harness, Shell<Game>, Game, Tile, Tile) {
    let (mut h, mut shell, mut game) = playing(name);
    game.sim.monsters.clear();
    let (x, y) = game.sim.player.tile;
    let (herb, cauldron) = ((x + 2, y), (x - 2, y));
    // Whatever the test map grew there makes way.
    game.sim.world.remove_prop(herb.0, herb.1);
    assert!(game.sim.world.place_prop(PropKind::HerbPurple, herb.0, herb.1));
    assert!(game.sim.world.place_prop(PropKind::Cauldron, cauldron.0, cauldron.1));
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game, herb, cauldron)
}

type Tile = (i32, i32);

fn bag_slot(h: &Harness, shell: &Shell<Game>, slot: usize) -> Vec2 {
    h.rect_of(body(shell, View::World).with("slot").with_index(slot)).expect("a bag slot").center()
}

fn slot_of(game: &Game, item: Item) -> usize {
    game.sim.inventory.slots().iter().position(|s| s.is_some_and(|s| s.item == item)).expect("in the bag")
}

#[test]
fn clicking_a_herb_picks_it_and_it_is_drawn_picked_until_it_grows_back() {
    let (mut h, mut shell, mut game, herb, _) = garden("alch_pick");
    let (whole, picked) = (game.sprites.id("herb_purple").unwrap(), game.sprites.id("herb_picked").unwrap());
    press_tile(&mut h, &mut shell, &mut game, herb);
    until(&mut h, &mut shell, &mut game, 20.0, |game| game.sim.is_spent(herb));
    pass(&mut h, &mut shell, &mut game, 1.0);
    assert_eq!(game.sim.inventory.count(Item::PurpleHerb), 2);
    assert_eq!(game.sim.player.tile, herb, "they stand on it to pick it");
    assert!(game.sim.skills.xp(Skill::Herbalism) > 0);
    assert!(game.hud.floating().contains(&"+1 Purple herb") || game.saves.read(0).unwrap().unwrap().inventory.count(Item::PurpleHerb) == 2);
    // Drawn: the picked plant where the flower was.
    let drawn_as = |game: &Game| {
        let mut scene = crate::scene::Scene::default();
        scene.build(&game.sim.world, &game.camera, game.view.size(), game.scale, &game.sprites, &game.look, &[], &|x, y| game.sim.is_spent((x, y)));
        let drawn = |id| scene.sprites.iter().filter(|s| s.uv == game.sprites.get(id).uv).count();
        (drawn(whole), drawn(picked))
    };
    let (flowers, stubs) = drawn_as(&game);
    assert!(stubs >= 1, "a picked plant is drawn");
    pass(&mut h, &mut shell, &mut game, 32.0);
    assert_eq!(game.sim.node_left(herb), Some(2), "grown back");
    assert_eq!(drawn_as(&game), (flowers + 1, stubs - 1));
}

#[test]
fn the_cauldron_brews_potions_under_alchemy() {
    let (mut h, mut shell, mut game, _, cauldron) = garden("alch_brew");
    game.sim.inventory.add(Item::PurpleHerb, 2);
    game.sim.inventory.add(Item::GoblinTooth, 2);
    press_tile(&mut h, &mut shell, &mut game, cauldron);
    until(&mut h, &mut shell, &mut game, 10.0, |game| game.sim.station().is_some());
    pass(&mut h, &mut shell, &mut game, 0.1);
    assert_eq!(game.sim.station(), Some(Station::Cauldron));
    let button = |h: &Harness, shell: &Shell<Game>, item: Item, label: &str| {
        let index = RECIPES.iter().position(|r| r.makes == item).unwrap();
        h.rect_of(body(shell, View::World).with("make").with_index(index).with(label))
    };
    assert!(button(&h, &shell, Item::BronzeSword, "1").is_none(), "nothing is forged in a cauldron");
    assert!(button(&h, &shell, Item::StrengthPotion, "1").is_none(), "nor is what a beginner cannot brew");
    let all = button(&h, &shell, Item::HealingPotion, "All").expect("a button").center();
    click(&mut h, &mut shell, &mut game, all);
    until(&mut h, &mut shell, &mut game, 30.0, |game| game.sim.making().is_none());
    assert_eq!(game.sim.inventory.count(Item::HealingPotion), 2);
    assert_eq!(game.sim.skills.xp(Skill::Alchemy), 40);
    assert_eq!(game.saves.read(0).unwrap().unwrap().inventory.count(Item::HealingPotion), 2, "saved as they are made");
}

#[test]
fn a_press_on_a_potion_in_the_bag_drinks_it_and_the_next_must_wait() {
    let (mut h, mut shell, mut game) = playing("alch_drink");
    game.sim.monsters.clear();
    game.sim.skills.set_xp(Skill::Vitality, sim::skills::xp_for_level(10));
    game.sim.inventory.add(Item::HealingPotion, 2);
    game.sim.player.health = 5;
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    let at = bag_slot(&h, &shell, slot_of(&game, Item::HealingPotion));
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.inventory.count(Item::HealingPotion), 1);
    // What it did is said on the next frame.
    turn(&mut h, &mut shell, &mut game);
    assert!(game.sim.player.health >= 20, "fifteen back at once: {}", game.sim.player.health);
    assert!(game.hud.floating().contains(&"+15"), "{:?}", game.hud.floating());
    assert_eq!(game.sim.player.goal(), None);
    assert_eq!(game.saves.read(0).unwrap().unwrap().inventory.count(Item::HealingPotion), 1, "saved at once");
    // The other one waits its few seconds.
    game.sim.player.health = 5;
    let at = bag_slot(&h, &shell, slot_of(&game, Item::HealingPotion));
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.inventory.count(Item::HealingPotion), 1, "not yet");
    pass(&mut h, &mut shell, &mut game, 6.5);
    game.sim.player.health = 5;
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.inventory.count(Item::HealingPotion), 0);
}

#[test]
fn a_lasting_potion_shows_under_the_health_bar_while_it_works() {
    let (mut h, mut shell, mut game) = playing("frame_potions");
    game.sim.monsters.clear();
    for item in [Item::StrengthPotion, Item::StoneskinPotion, Item::HealingPotion, Item::HealingPotion, Item::RegenerationPotion, Item::PurpleHerb, Item::CyanHerb] {
        game.sim.inventory.add(item, 1);
    }
    game.sim.player.health = 3;
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    let Some(before) = snap("frame-potions-before", &mut h, &mut shell, &mut game) else { return };
    for item in [Item::StrengthPotion, Item::StoneskinPotion, Item::HealingPotion] {
        let at = bag_slot(&h, &shell, slot_of(&game, item));
        click(&mut h, &mut shell, &mut game, at);
    }
    assert!(game.sim.potion_left(Potion::Strength) > 0 && game.sim.potion_left(Potion::Stoneskin) > 0);
    assert_eq!(game.sim.max_hit(), 4, "two more than bare hands");
    let Some(image) = snap("frame-potions", &mut h, &mut shell, &mut game) else { return };
    // Under the health bar: two plates with the potions on them, where
    // there was only grass.
    let under = Rect::from_min_size(game.view.min + Vec2::new(10.0, 60.0), Vec2::new(260.0, 55.0));
    let plate = [0xc9, 0xb6, 0x8a];
    assert_eq!(count(&before, under, plate, 2), 0);
    assert!(count(&image, under, plate, 2) > 800, "the potions at work: {}", count(&image, under, plate, 2));
    assert!(count(&image, under, [255, 255, 255], 30) > 60, "and their seconds");
    // The healing potion left in the bag is dark, waiting.
    let waiting = h.rect_of(body(&shell, View::World).with("slot").with_index(slot_of(&game, Item::HealingPotion))).unwrap();
    assert!(count(&image, waiting, [0xd9, 0x48, 0x3b], 4) == 0, "its red is dimmed while its kind waits");
    // What they do wears off, and the row goes with it.
    pass(&mut h, &mut shell, &mut game, 61.0);
    assert_eq!(game.sim.max_hit(), 2);
    let after = snap("frame-potions-after", &mut h, &mut shell, &mut game).unwrap();
    assert_eq!(count(&after, under, plate, 2), 0);
}

#[test]
fn the_editor_lists_the_cauldron() {
    let (mut h, mut shell, mut game) = playing("alch_editor");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    panel_tab(&mut h, &mut shell, &mut game, 1);
    let row = prop_row(&h, &shell, PropKind::Cauldron);
    click(&mut h, &mut shell, &mut game, row);
    assert_eq!((game.editor.tool, game.editor.prop), (Tool::Place, PropKind::Cauldron));
    let (x, y) = game.sim.player.tile;
    let at = screen_of(&game, (x + 2, y + 1));
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.world.prop_at(x + 2, y + 1).map(|p| p.kind), Some(PropKind::Cauldron));
}
