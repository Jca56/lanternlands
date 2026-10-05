//! Maps saved to a real folder and read back.

use std::fs;
use std::path::PathBuf;

use sim::{Marker, PersonKind, PropKind, Terrain, Tile, World};

/// A fresh folder under the build's own scratch space.
fn folder(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn files(dir: &PathBuf) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir).expect("the folder is there").map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    names
}

/// Every tile over the test map's stretch and a margin around it.
fn snapshot(world: &World) -> Vec<Tile> {
    (-70..100).flat_map(|y| (-70..100).map(move |x| (x, y))).map(|(x, y)| world.tile(x, y)).collect()
}

#[test]
fn the_test_map_comes_back_tile_for_tile() {
    let dir = folder("round_trip");
    let mut world = sim::testmap::build();
    assert_eq!(world.save(&dir), Ok(2));
    assert_eq!(files(&dir), ["0_0.chunk", "1_0.chunk"]);
    assert!(!world.is_dirty());
    let back = World::load(&dir).expect("loads");
    assert!(!back.is_dirty(), "a map just loaded has nothing to save");
    assert_eq!(snapshot(&back), snapshot(&world));
    assert_eq!(back.props_in(-70, -70, 100, 100).count(), world.props_in(-70, -70, 100, 100).count());
}

#[test]
fn only_what_changed_is_written() {
    let dir = folder("only_changed");
    let mut world = sim::testmap::build();
    world.save(&dir).unwrap();
    assert_eq!(world.save(&dir), Ok(0), "nothing changed");
    let untouched = fs::read(dir.join("1_0.chunk")).unwrap();
    world.set_terrain(3, 3, Terrain::Water);
    world.set_terrain(-1, -40, Terrain::Sand);
    world.place_prop(PropKind::Rock, -1, -40);
    assert_eq!(world.save(&dir), Ok(2));
    assert_eq!(files(&dir), ["-1_-2.chunk", "0_0.chunk", "1_0.chunk"]);
    assert_eq!(fs::read(dir.join("1_0.chunk")).unwrap(), untouched);
    let back = World::load(&dir).unwrap();
    assert_eq!(back.tile(3, 3).terrain, Some(Terrain::Water));
    assert_eq!(back.tile(-1, -40), Tile { terrain: Some(Terrain::Sand), prop: Some(PropKind::Rock), marker: None, spawn: None, person: None });
}

#[test]
fn a_chunk_erased_to_nothing_loses_its_file() {
    let dir = folder("erased");
    let mut world = World::new();
    world.set_terrain(0, 0, Terrain::Grass);
    world.set_terrain(40, 0, Terrain::Grass);
    world.save(&dir).unwrap();
    assert_eq!(files(&dir), ["0_0.chunk", "1_0.chunk"]);
    world.clear_tile(40, 0);
    assert_eq!(world.save(&dir), Ok(1));
    assert_eq!(files(&dir), ["0_0.chunk"]);
    assert_eq!(world.chunk_count(), 1);
    assert_eq!(World::load(&dir).unwrap().bounds(), Some([0, 0, 32, 32]));
}

#[test]
fn no_scratch_files_are_left_and_strangers_are_ignored() {
    let dir = folder("strangers");
    let mut world = sim::testmap::build();
    world.save(&dir).unwrap();
    assert!(files(&dir).iter().all(|name| name.ends_with(".chunk")), "{:?}", files(&dir));
    fs::write(dir.join("notes.txt"), "not a chunk").unwrap();
    fs::write(dir.join(".0_0.chunk.tmp"), "half a chunk").unwrap();
    assert_eq!(snapshot(&World::load(&dir).expect("loads")), snapshot(&world));
}

#[test]
fn a_folder_that_is_not_there_is_an_empty_world() {
    let world = World::load(&folder("never_made")).expect("loads");
    assert_eq!(world.bounds(), None);
}

#[test]
fn a_bad_chunk_file_fails_the_load_and_names_the_file() {
    let dir = folder("bad");
    let mut world = sim::testmap::build();
    world.save(&dir).unwrap();
    fs::write(dir.join("1_0.chunk"), b"CHNK then rubbish").unwrap();
    let why = World::load(&dir).err().expect("refused");
    assert!(why.contains("1_0.chunk"), "{why}");
    fs::write(dir.join("1_0.chunk"), fs::read(dir.join("0_0.chunk")).unwrap()).unwrap();
    fs::write(dir.join("east.chunk"), b"").unwrap();
    let why = World::load(&dir).err().expect("refused");
    assert!(why.contains("east.chunk") && why.contains("name"), "{why}");
}

#[test]
fn the_player_start_is_saved_with_the_map_and_moves_between_chunk_files() {
    let dir = folder("start");
    let mut world = sim::testmap::build();
    assert!(world.set_marker(Marker::PlayerStart, 24, 14));
    world.save(&dir).unwrap();
    assert_eq!(World::load(&dir).unwrap().marker(Marker::PlayerStart), Some((24, 14)));
    // Moved into the other chunk: both files are rewritten, one start is left.
    assert!(world.set_marker(Marker::PlayerStart, 40, 5));
    assert_eq!(world.save(&dir), Ok(2));
    let back = World::load(&dir).unwrap();
    assert_eq!(back.marker(Marker::PlayerStart), Some((40, 5)));
    assert_eq!(back.marker_at(24, 14), None);
}

#[test]
fn a_map_with_two_player_starts_is_refused() {
    let (one, two) = (folder("two_starts_a"), folder("two_starts"));
    let mut world = sim::testmap::build();
    world.set_marker(Marker::PlayerStart, 5, 5);
    world.save(&one).unwrap();
    let mut other = sim::testmap::build();
    other.set_marker(Marker::PlayerStart, 40, 5);
    other.save(&two).unwrap();
    // One chunk from each map: a start in both.
    fs::copy(one.join("0_0.chunk"), two.join("0_0.chunk")).unwrap();
    let why = World::load(&two).err().expect("refused");
    assert!(why.contains("2 player_start markers"), "{why}");
}

#[test]
fn the_people_are_saved_with_the_map_and_only_their_chunk_is_rewritten() {
    let dir = folder("people");
    let mut world = sim::testmap::build();
    world.save(&dir).unwrap();
    assert!(world.set_person(24, 14, Some(PersonKind::Smith)) && world.set_person(26, 15, Some(PersonKind::Guard)));
    assert_eq!(world.save(&dir), Ok(1));
    let back = World::load(&dir).unwrap();
    assert_eq!(back.people(), vec![(PersonKind::Smith, 24, 14), (PersonKind::Guard, 26, 15)]);
    assert_eq!(snapshot(&back), snapshot(&world));
}
