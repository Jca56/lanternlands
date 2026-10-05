//! Putting gear on and taking it off, and how it is drawn on the
//! player, with no window.

use sim::{Metal, Slot};

use super::frame::{body, count, snap};
use super::play::{pass, playing, pose_of};
use super::*;
use crate::scene::{GEAR_ORDER, metal_tint};
use crate::wear::grip_tint;

const SET: [Item; 7] = [Item::BronzeSword, Item::BronzeShield, Item::BronzeHelmet, Item::BronzeChestplate, Item::BronzeGreaves, Item::BronzeGauntlets, Item::BronzeBoots];
/// The bronze a piece's main grey comes out as, and the player's own
/// tunic blue.
const BRONZE: [u8; 3] = [0xb6, 0x82, 0x49];
const TUNIC: [u8; 3] = [0x44, 0x53, 0x65];

/// The game with the whole bronze set in the bag, and nothing about to
/// bother the player.
fn with_the_set(name: &str) -> (Harness, Shell<Game>, Game) {
    let (mut h, mut shell, mut game) = playing(name);
    game.sim.monsters.clear();
    for item in SET {
        game.sim.inventory.add(item, 1);
    }
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game)
}

fn bag_slot(h: &Harness, shell: &Shell<Game>, slot: usize) -> Vec2 {
    h.rect_of(body(shell, View::World).with("slot").with_index(slot)).expect("a bag slot").center()
}

fn gear_slot(h: &Harness, shell: &Shell<Game>, slot: Slot) -> Option<Rect> {
    h.rect_of(body(shell, View::World).with("gear").with(slot.name()))
}

#[test]
fn a_press_on_gear_in_the_bag_puts_it_on_and_a_press_on_it_worn_takes_it_off() {
    let (mut h, mut shell, mut game) = with_the_set("gear_wear");
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('g'), Modifiers::NONE);
    assert!(game.hud.gear_open && game.hud.bag_open);
    for slot in Slot::ALL {
        assert!(gear_slot(&h, &shell, slot).is_some_and(|r| r.width() >= 90.0), "a big slot for the {}", slot.name());
    }
    // The sword is in the bag's first slot.
    let at = bag_slot(&h, &shell, 0);
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.gear.worn(Slot::Weapon), Some(Item::BronzeSword));
    assert_eq!(game.sim.inventory.count(Item::BronzeSword), 0);
    assert_eq!(game.sim.player.goal(), None, "and nobody walked anywhere");
    assert_eq!(game.saves.read(0).unwrap().unwrap().gear.worn(Slot::Weapon), Some(Item::BronzeSword), "saved at once");
    // Off again, back into the bag.
    let at = gear_slot(&h, &shell, Slot::Weapon).unwrap().center();
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!((game.sim.gear.worn(Slot::Weapon), game.sim.inventory.count(Item::BronzeSword)), (None, 1));
    assert_eq!(game.saves.read(0).unwrap().unwrap().gear.worn(Slot::Weapon), None);
    // Ore is not worn: a press on it does nothing.
    game.sim.inventory.add(Item::CopperOre, 3);
    turn(&mut h, &mut shell, &mut game);
    let ore = game.sim.inventory.slots().iter().position(|s| s.is_some_and(|s| s.item == Item::CopperOre)).unwrap();
    let at = bag_slot(&h, &shell, ore);
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.inventory.count(Item::CopperOre), 3);
}

#[test]
fn the_gear_button_and_the_g_key_open_and_close_the_panel() {
    let (mut h, mut shell, mut game) = playing("gear_panel");
    assert!(!game.hud.gear_open && gear_slot(&h, &shell, Slot::Head).is_none());
    let button = h.rect_of(body(&shell, View::World).with("Gear")).expect("a Gear button");
    assert!(button.width() >= 100.0 && button.height() >= 50.0, "a big one");
    click(&mut h, &mut shell, &mut game, button.center());
    assert!(game.hud.gear_open);
    assert_eq!(game.sim.player.goal(), None);
    key(&mut h, &mut shell, &mut game, Key::Char('g'), Modifiers::NONE);
    assert!(!game.hud.gear_open);
    // Not in the editor.
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('g'), Modifiers::NONE);
    assert!(!game.hud.gear_open);
}

#[test]
fn what_is_worn_is_drawn_in_layers_over_the_body_in_its_metals_colour() {
    let (mut h, mut shell, mut game) = with_the_set("gear_layers");
    let bare = game.extras().len();
    for slot in 0..SET.len() {
        game.sim.equip(slot);
    }
    turn(&mut h, &mut shell, &mut game);
    let bronze = metal_tint(Metal::Bronze);
    for facing in [Facing::Down, Facing::Up, Facing::Right, Facing::Left] {
        game.facing = facing;
        let extras = game.extras();
        let (body, flip) = game.look.player.sprite(facing, Pose::Idle).unwrap();
        let at = extras.iter().position(|p| p.sprite == body).expect("the body is drawn");
        // Everything drawn for the player stands on the same ground and
        // is mirrored with them.
        let ground = extras[at].ground;
        let layers: Vec<_> = extras.iter().filter(|p| p.ground == ground).collect();
        assert!(layers.iter().all(|p| p.flip == flip));
        assert!(layers.len() >= 7, "{facing:?}: {} layers", layers.len());
        // The hand over the sword's grip is one more layer, in its
        // gauntlet's colour: there from the front and the side, where
        // the sword is in front of the body.
        let grip = game.look.grip.sprite(facing, Pose::Idle).map(|(id, _)| id);
        assert_eq!(grip.is_some(), facing != Facing::Up, "{facing:?}");
        assert!(layers.iter().filter(|p| p.sprite != body && Some(p.sprite) != grip).all(|p| p.tint == bronze), "gear is tinted, the body is not");
        assert!(layers.iter().filter(|p| Some(p.sprite) == grip).all(|p| p.tint == grip_tint(Some(bronze))));
        assert_eq!(extras[at].tint, [1.0; 4]);
        // Over the body in the order that reads right; the shield shows
        // from behind it when the player is seen from the back or side.
        let over: Vec<_> = extras[at + 1..].iter().map(|p| p.sprite).collect();
        let expected: Vec<_> = GEAR_ORDER.iter().filter_map(|slot| game.look.gear(*slot).over.sprite(facing, Pose::Idle)).map(|(id, _)| id).chain(grip).collect();
        assert_eq!(over, expected, "{facing:?}");
        // The sword itself is behind the body when seen from the back.
        assert_eq!(game.look.gear(Slot::Weapon).under.sprite(facing, Pose::Idle).is_some(), facing == Facing::Up, "{facing:?}");
        let behind = game.look.gear(Slot::Shield).under.sprite(facing, Pose::Idle);
        assert_eq!(behind.is_some(), facing != Facing::Down, "{facing:?}");
        if let Some((shield, _)) = behind {
            assert!(extras[..at].iter().any(|p| p.sprite == shield), "{facing:?}: the shield is drawn before the body");
        }
        assert!(extras.len() > bare);
    }
    // Taking it all off leaves the body alone again.
    for slot in Slot::ALL {
        game.sim.unequip(slot);
    }
    assert_eq!(game.extras().len(), bare);
    assert_eq!(pose_of(&game), Some(Pose::Idle));
}

#[test]
fn the_armoured_player_is_bronze_where_they_were_blue_from_every_side() {
    let (mut h, mut shell, mut game) = with_the_set("frame_gear");
    game.sim.inventory.add(Item::CopperOre, 2);
    let Some(plain) = snap("frame-gear-none", &mut h, &mut shell, &mut game) else { return };
    let px = game.camera.tile_px(game.scale);
    let feet = game.camera.to_screen(game.player_ground(), game.view, game.scale);
    let figure = Rect::new(feet - Vec2::new(px * 0.9, px * 2.0), feet + Vec2::new(px * 0.9, px * 0.3));
    assert!(count(&plain, figure, TUNIC, 4) > 1000, "blue to begin with: {}", count(&plain, figure, TUNIC, 4));
    assert_eq!(count(&plain, figure, BRONZE, 6), 0);
    for slot in 0..SET.len() {
        game.sim.equip(slot);
    }
    key(&mut h, &mut shell, &mut game, Key::Char('g'), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('i'), Modifiers::NONE);
    for (facing, name) in [(Facing::Down, "frame-gear-down"), (Facing::Up, "frame-gear-up"), (Facing::Right, "frame-gear-right"), (Facing::Left, "frame-gear-left")] {
        game.facing = facing;
        let Some(image) = snap(name, &mut h, &mut shell, &mut game) else { return };
        assert!(count(&image, figure, BRONZE, 6) > 1500, "{facing:?}: bronze all over: {}", count(&image, figure, BRONZE, 6));
        assert_eq!(count(&image, figure, TUNIC, 4), 0, "{facing:?}: no tunic shows through the chestplate");
    }
    // The gear panel shows all seven pieces on their plates, and what
    // they add up to.
    let head = gear_slot(&h, &shell, Slot::Head).expect("the panel is open");
    let image = snap("frame-gear-panel", &mut h, &mut shell, &mut game).unwrap();
    assert!(count(&image, head, [0xc9, 0xb6, 0x8a], 2) > 1000, "the helmet's plate");
    assert!(count(&image, head, [0xb8, 0x7a, 0x3d], 6) > 500, "and the helmet on it");
    // Reaching over the hotbar, the panel stands on it and hides none of it.
    let (feet, hot) = (gear_slot(&h, &shell, Slot::Feet).unwrap(), crate::hotbar::SLOTS - 1);
    let last = h.rect_of(body(&shell, View::World).with("ability").with_index(hot)).expect("the hotbar's last button");
    assert!(feet.max.y < last.min.y, "the gear panel ends above the hotbar: {feet:?} against {last:?}");
    pass(&mut h, &mut shell, &mut game, 0.1);
}
