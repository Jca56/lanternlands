//! People and talking, driven with no window.

use sim::{Metal, PersonKind};

use super::frame::{GRASS, INK, body, count, snap};
use super::play::{pass, playing, screen_of};
use super::*;
use crate::scene::{Placed, metal_tint};
use crate::talk;

/// The apron's leather and the guard's bronze, as a piece of gear's
/// main grey comes out in each.
const LEATHER: [u8; 3] = [0x61, 0x42, 0x27];
const BRONZE: [u8; 3] = [0xb6, 0x82, 0x49];

/// The game with the player on open grass and someone living `east`
/// tiles to their east.
pub(super) fn with_person(name: &str, kind: PersonKind, east: i32) -> (Harness, Shell<Game>, Game) {
    let (mut h, mut shell, mut game) = playing(name);
    let (x, y) = game.sim.player.tile;
    assert!(game.sim.world.set_person(x + east, y, Some(kind)));
    game.sim.settle_player();
    turn(&mut h, &mut shell, &mut game);
    (h, shell, game)
}

/// Where a person's body is on screen: half a tile over their feet.
pub(super) fn body_of(game: &Game, person: usize) -> Vec2 {
    game.camera.to_screen(game.person_grounds()[person] - Vec2::new(0.0, 0.5), game.view, game.scale)
}

/// Where something in the talk box is on screen.
fn in_the_box(h: &Harness, shell: &Shell<Game>, what: &str) -> Rect {
    h.rect_of(body(shell, View::World).with(what)).unwrap_or_else(|| panic!("nothing called {what} in the talk box"))
}

/// Click on a person and let time pass until they are talking.
pub(super) fn hail(h: &mut Harness, shell: &mut Shell<Game>, game: &mut Game, person: usize) {
    let at = body_of(game, person);
    click(h, shell, game, at);
    assert_eq!(game.sim.awaited(), Some(person), "the click was on them");
    for _ in 0..200 {
        if game.sim.saying().is_some() {
            break;
        }
        pass(h, shell, game, 0.05);
    }
    // The last step of the way finishes, and the box is laid out.
    pass(h, shell, game, 0.6);
    assert!(game.sim.saying().is_some(), "they are talking");
}

/// What is drawn standing on a point of the world.
fn standing_on(game: &Game, ground: Vec2) -> Vec<Placed> {
    game.extras().into_iter().filter(|p| (p.ground - ground).length() < 0.001).collect()
}

#[test]
fn a_click_on_someone_walks_up_to_them_and_they_talk_a_line_at_a_time() {
    let (mut h, mut shell, mut game) = with_person("talk_click", PersonKind::Smith, 4);
    let lines = PersonKind::Smith.lines();
    let at = body_of(&game, 0);
    assert_eq!(game.person_under(at), Some(0));
    let tile_px = game.camera.tile_px(game.scale);
    assert_eq!(game.person_under(at + Vec2::new(tile_px, 0.0)), None, "the grass beside them is not them");
    hail(&mut h, &mut shell, &mut game, 0);
    let (me, them) = (game.sim.player.tile, game.sim.people[0].tile);
    assert_eq!((me.0, me.1), (them.0 - 1, them.1), "beside them, on the side they came from");
    assert_eq!((game.facing, game.person_facing[0]), (Facing::Right, Facing::Left), "face to face");
    assert_eq!(game.sim.saying().map(|s| s.text), Some(lines[0]));

    // A big box with a big button, clear of the hotbar.
    let (panel, next) = (in_the_box(&h, &shell, "talk"), in_the_box(&h, &shell, talk::NEXT));
    assert!(panel.width() >= 800.0 && panel.max.y < WINDOW.y - 110.0, "{panel:?}");
    assert!(next.width() >= 150.0 && next.height() >= 55.0 && panel.contains(next.center()), "{next:?}");

    // Space hears the next line; a press on the button, the one after.
    key(&mut h, &mut shell, &mut game, Key::Space, Modifiers::NONE);
    assert_eq!(game.sim.saying().map(|s| s.text), Some(lines[1]));
    click(&mut h, &mut shell, &mut game, next.center());
    assert_eq!(game.sim.saying().map(|s| (s.text, s.line, s.of)), Some((lines[2], 3, 3)));
    // The frame the press asked for.
    turn(&mut h, &mut shell, &mut game);
    // After the last line the button says goodbye, and that is that.
    let done = in_the_box(&h, &shell, talk::DONE);
    click(&mut h, &mut shell, &mut game, done.center());
    assert_eq!(game.sim.saying(), None);
    assert_eq!((game.sim.player.tile, game.sim.player.goal()), (me, None), "nobody walked anywhere");
}

#[test]
fn a_press_on_the_box_itself_hears_on_and_walks_nobody_anywhere() {
    let (mut h, mut shell, mut game) = with_person("talk_box", PersonKind::Herbalist, 3);
    hail(&mut h, &mut shell, &mut game, 0);
    let me = game.sim.player.tile;
    let panel = in_the_box(&h, &shell, "talk");
    click(&mut h, &mut shell, &mut game, panel.min + Vec2::new(60.0, 40.0));
    assert_eq!(game.sim.saying().map(|s| s.line), Some(2));
    key(&mut h, &mut shell, &mut game, Key::Enter, Modifiers::NONE);
    assert_eq!(game.sim.saying().map(|s| s.line), Some(3));
    assert_eq!((game.sim.player.tile, game.sim.player.goal()), (me, None), "the world under the box was not clicked");
    // With nobody talking, the same keys do nothing at all.
    key(&mut h, &mut shell, &mut game, Key::Space, Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Space, Modifiers::NONE);
    assert_eq!((game.sim.saying(), game.mode), (None, Mode::Playing { slot: 0 }));
}

#[test]
fn esc_ends_a_talk_before_it_leaves_the_game_and_so_does_walking_off() {
    let (mut h, mut shell, mut game) = with_person("talk_leave", PersonKind::Smith, 3);
    hail(&mut h, &mut shell, &mut game, 0);
    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert_eq!((game.sim.saying(), game.mode), (None, Mode::Playing { slot: 0 }));

    hail(&mut h, &mut shell, &mut game, 0);
    let (x, y) = game.sim.player.tile;
    let away = screen_of(&game, (x - 2, y - 2));
    click(&mut h, &mut shell, &mut game, away);
    assert_eq!(game.sim.saying(), None);
    assert_eq!(game.sim.player.goal(), Some((x - 2, y - 2)));

    key(&mut h, &mut shell, &mut game, Key::Escape, Modifiers::NONE);
    assert_eq!(game.mode, Mode::Title, "with nothing open, Esc is the way out");
}

#[test]
fn people_are_drawn_dressed_where_they_stand_under_the_player_in_the_list() {
    let (_, _, mut game) = with_person("people_dressed", PersonKind::Guard, 3);
    let (x, y) = game.sim.player.tile;
    assert!(game.sim.world.set_person(x - 3, y, Some(PersonKind::Smith)));
    game.sim.settle_player();
    // Row by row, the smith is first.
    assert_eq!(game.sim.people.iter().map(|p| p.kind).collect::<Vec<_>>(), vec![PersonKind::Smith, PersonKind::Guard]);
    let grounds = game.person_grounds();
    let idle = |kind| game.look.person(kind).sprite(Facing::Down, Pose::Idle).expect("drawn").0;

    let guard = standing_on(&game, grounds[1]);
    let bronze = metal_tint(Metal::Bronze);
    assert_eq!(guard.iter().filter(|p| p.sprite == idle(PersonKind::Guard)).count(), 1, "their own body");
    assert!(guard.iter().filter(|p| p.tint == bronze).count() >= 7, "and bronze over it from head to foot: {}", guard.len());
    let smith = standing_on(&game, grounds[0]);
    assert_eq!((smith.len(), smith[0].sprite), (3, idle(PersonKind::Smith)), "a body, then an apron and gloves over it");
    assert!(smith[1..].iter().all(|p| p.tint != [1.0; 4] && p.tint != bronze));
    // The player is still the last thing stood in the scene.
    let (me, _) = game.look.player.sprite(Facing::Down, Pose::Idle).unwrap();
    assert_eq!(game.extras().last().map(|p| p.sprite), Some(me));
}

/// The pose a person is drawn in.
fn pose_of(game: &Game, person: usize) -> Option<Pose> {
    let (sprites, facing) = (game.look.person(game.sim.people[person].kind), game.person_facing[person]);
    let extras = game.extras();
    Pose::ALL.into_iter().find(|&pose| sprites.sprite(facing, pose).is_some_and(|(id, flip)| extras.iter().any(|p| p.sprite == id && p.flip == flip)))
}

#[test]
fn the_guard_walks_their_round_at_a_walk_and_stops_for_whoever_hails_them() {
    let (mut h, mut shell, mut game) = with_person("people_round", PersonKind::Guard, 5);
    let home = game.sim.people[0].home;
    let (mut stood, mut poses) = (std::collections::HashSet::new(), Vec::new());
    for _ in 0..1200 {
        h.advance(0.05);
        turn(&mut h, &mut shell, &mut game);
        let guard = &game.sim.people[0];
        assert!((guard.tile.0 - home.0).abs() <= 2 && (guard.tile.1 - home.1).abs() <= 2, "they strayed to {:?}", guard.tile);
        stood.insert(guard.tile);
        let pose = pose_of(&game, 0).expect("the guard is drawn");
        if !poses.contains(&pose) {
            poses.push(pose);
        }
    }
    assert!(stood.len() > 2, "nobody was playing, and they walked their round: {stood:?}");
    assert!(poses.contains(&Pose::Idle) && poses.contains(&Pose::Pass) && (poses.contains(&Pose::StepA) || poses.contains(&Pose::StepB)), "at a walk: {poses:?}");
    assert!(!game.sim.player.busy());

    // Hailed, they stand and wait, turned to whoever is coming.
    let at = body_of(&game, 0);
    click(&mut h, &mut shell, &mut game, at);
    let waiting = game.sim.people[0].tile;
    for _ in 0..200 {
        pass(&mut h, &mut shell, &mut game, 0.05);
        assert_eq!(game.sim.people[0].tile, waiting);
    }
    assert_eq!(game.sim.saying().map(|s| s.kind), Some(PersonKind::Guard));
    let (me, them) = (game.sim.player.tile, game.sim.people[0].tile);
    assert_eq!(Some(game.person_facing[0]), Facing::of_step(me.0 - them.0, me.1 - them.1));
    assert_eq!(pose_of(&game, 0), Some(Pose::Idle));
}

#[test]
fn the_person_tool_settles_someone_who_is_there_once_the_editor_closes() {
    let (mut h, mut shell, mut game) = playing("people_tool");
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    (game.editor.tool, game.editor.person) = (Tool::Person, PersonKind::Herbalist);
    let (x, y) = game.sim.player.tile;
    let home = (x + 2, y + 1);
    let at = screen_of(&game, home);
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.world.people(), vec![(PersonKind::Herbalist, home.0, home.1)]);
    // Editing, a ghost of them stands on their home, and nobody yet.
    let (idle, _) = game.look.person(PersonKind::Herbalist).sprite(Facing::Down, Pose::Idle).unwrap();
    let ghosts: Vec<Placed> = game.extras().into_iter().filter(|p| p.sprite == idle).collect();
    assert_eq!(ghosts.len(), 1);
    assert!(ghosts[0].tint[3] < 1.0 && ghosts[0].ground == Vec2::new(home.0 as f64 + 0.5, home.1 as f64 + 0.5));
    assert!(game.sim.people.is_empty());
    // One undo takes them away again, and redo brings them back.
    key(&mut h, &mut shell, &mut game, Key::Char('z'), Modifiers::CTRL);
    assert_eq!(game.sim.world.people(), vec![]);
    key(&mut h, &mut shell, &mut game, Key::Char('y'), Modifiers::CTRL);

    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    assert_eq!(game.sim.people.iter().map(|p| (p.kind, p.tile)).collect::<Vec<_>>(), vec![(PersonKind::Herbalist, home)]);
    // And they are saved with the map like everything else.
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    key(&mut h, &mut shell, &mut game, Key::Char('s'), Modifiers::CTRL);
    assert_eq!(World::load(&game.map).unwrap().people(), vec![(PersonKind::Herbalist, home.0, home.1)]);
    // The remove tool takes whoever lives on a tile.
    game.editor.tool = Tool::Remove;
    let at = screen_of(&game, home);
    click(&mut h, &mut shell, &mut game, at);
    assert_eq!(game.sim.world.people(), vec![]);
}

/// The square of screen someone standing on `ground` is drawn in.
fn around(game: &Game, ground: Vec2) -> Rect {
    let px = game.camera.tile_px(game.scale);
    let feet = game.camera.to_screen(ground, game.view, game.scale);
    Rect::from_center_size(feet - Vec2::new(0.0, px * 0.6), Vec2::new(px * 1.4, px * 1.6))
}

#[test]
fn a_talk_is_drawn_in_its_box_over_the_world_with_names_over_heads() {
    let (mut h, mut shell, mut game) = with_person("frame_talk", PersonKind::Smith, 2);
    let (x, y) = game.sim.player.tile;
    assert!(game.sim.world.set_person(x - 3, y - 1, Some(PersonKind::Guard)));
    game.sim.settle_player();
    turn(&mut h, &mut shell, &mut game);
    let smith = game.sim.people.iter().position(|p| p.kind == PersonKind::Smith).unwrap();
    hail(&mut h, &mut shell, &mut game, smith);
    let Some(image) = snap("frame-talk", &mut h, &mut shell, &mut game) else { return };

    // The box covers the world under it, and has writing in it.
    let panel = in_the_box(&h, &shell, "talk");
    assert_eq!(count(&image, panel.shrink(4.0), GRASS, 1), 0, "no grass shows through the box");
    assert!(count(&image, panel, INK, 14) > 1500, "what they say is written in it: {}", count(&image, panel, INK, 14));
    // The smith in their apron, and the guard in bronze, where they stand.
    let grounds = game.person_grounds();
    let guard = 1 - smith;
    assert!(count(&image, around(&game, grounds[smith]), LEATHER, 6) > 300, "the apron: {}", count(&image, around(&game, grounds[smith]), LEATHER, 6));
    assert!(count(&image, around(&game, grounds[guard]), BRONZE, 6) > 600, "the armour: {}", count(&image, around(&game, grounds[guard]), BRONZE, 6));
    assert_eq!(count(&image, around(&game, grounds[smith]), BRONZE, 6), 0, "and no bronze on the smith");
    // Each has their name in white over their head.
    let px = game.camera.tile_px(game.scale);
    for &feet in &grounds {
        let over = Rect::from_center_size(game.camera.to_screen(feet - Vec2::new(0.0, 1.45), game.view, game.scale) - Vec2::new(0.0, 28.0), Vec2::new(px * 1.2, 50.0));
        assert!(count(&image, over, [255, 255, 255], 30) > 60, "a name over their head: {}", count(&image, over, [255, 255, 255], 30));
    }
}

#[test]
fn the_editor_lists_the_people_and_shows_a_ghost_where_one_lives() {
    let (mut h, mut shell, mut game) = playing("frame_people_editor");
    let (x, y) = game.sim.player.tile;
    assert!(game.sim.world.set_person(x + 2, y, Some(PersonKind::Herbalist)));
    key(&mut h, &mut shell, &mut game, Key::F(2), Modifiers::NONE);
    panel_tab(&mut h, &mut shell, &mut game, 2);
    let Some(image) = snap("frame-editor-people", &mut h, &mut shell, &mut game) else { return };
    // Her green tunic is in her row, and nobody else's.
    const GREEN: [u8; 3] = [0x4b, 0x7f, 0x39];
    let row = |kind| h.rect_of(body(&shell, View::Editor).with("map editor").with(panel::PEOPLE).with(&panel::person_label(kind))).expect("a row for them");
    assert!(count(&image, row(PersonKind::Herbalist), GREEN, 8) > 40, "her picture is in her row: {}", count(&image, row(PersonKind::Herbalist), GREEN, 8));
    assert_eq!(count(&image, row(PersonKind::Smith), GREEN, 8), 0);
    // The ghost on her home: something drawn there, thinned by the grass behind it.
    let ghost = around(&game, Vec2::new((x + 2) as f64 + 0.5, y as f64 + 0.5));
    assert!(count(&image, ghost, GRASS, 1) < (ghost.area() * 0.9) as usize, "something is drawn on her home");
    assert_eq!(count(&image, ghost, GREEN, 2), 0, "and it is a ghost, not the full colour");
}
