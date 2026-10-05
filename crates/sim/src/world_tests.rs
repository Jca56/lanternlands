//! Tests for the world's tiles, props, markers and monsters' homes.

use super::*;

#[test]
fn a_tile_is_void_until_it_is_set() {
    let mut world = World::new();
    assert_eq!(world.terrain(3, 4), None);
    world.set_terrain(3, 4, Terrain::Sand);
    assert_eq!(world.terrain(3, 4), Some(Terrain::Sand));
    // The rest of its chunk is still void.
    assert_eq!(world.terrain(4, 4), None);
    world.clear_tile(3, 4);
    assert_eq!(world.terrain(3, 4), None);
}

#[test]
fn negative_tiles_land_in_their_own_chunks() {
    let mut world = World::new();
    world.set_terrain(-1, -1, Terrain::Water);
    world.set_terrain(0, 0, Terrain::Dirt);
    world.set_terrain(-CHUNK, -CHUNK, Terrain::Sand);
    assert_eq!(world.terrain(-1, -1), Some(Terrain::Water));
    assert_eq!(world.terrain(0, 0), Some(Terrain::Dirt));
    assert_eq!(world.terrain(-CHUNK, -CHUNK), Some(Terrain::Sand));
    assert_eq!(world.terrain(-CHUNK - 1, -CHUNK), None);
    assert_eq!(world.bounds(), Some([-CHUNK, -CHUNK, CHUNK, CHUNK]));
}

#[test]
fn one_prop_per_tile_and_only_on_ground() {
    let mut world = World::new();
    assert!(!world.place_prop(PropKind::Tree, 5, 5), "no ground");
    world.set_terrain(5, 5, Terrain::Grass);
    assert!(world.place_prop(PropKind::Tree, 5, 5));
    assert!(!world.place_prop(PropKind::Rock, 5, 5), "taken");
    assert_eq!(world.prop_at(5, 5).map(|p| p.kind), Some(PropKind::Tree));
    assert_eq!(world.remove_prop(5, 5).map(|p| p.kind), Some(PropKind::Tree));
    assert_eq!(world.prop_at(5, 5), None);
}

#[test]
fn clearing_a_tile_takes_its_prop() {
    let mut world = World::new();
    world.set_terrain(1, 1, Terrain::Grass);
    world.place_prop(PropKind::Rock, 1, 1);
    world.clear_tile(1, 1);
    assert_eq!(world.prop_at(1, 1), None);
}

#[test]
fn props_in_a_box_cross_chunks_and_respect_its_edges() {
    let mut world = World::new();
    let spots = [(-40, 0), (-1, -1), (0, 0), (31, 31), (32, 32), (70, 10)];
    for &(x, y) in &spots {
        world.set_terrain(x, y, Terrain::Grass);
        assert!(world.place_prop(PropKind::Tree, x, y));
    }
    let mut found: Vec<(i32, i32)> = world.props_in(-1, -1, 32, 32).map(|p| (p.x, p.y)).collect();
    found.sort();
    assert_eq!(found, vec![(-1, -1), (0, 0), (31, 31)]);
    assert_eq!(world.props_in(-100, -100, 100, 100).count(), spots.len());
    assert_eq!(world.props_in(5, 5, 6, 6).count(), 0);
}

#[test]
fn an_empty_world_has_no_bounds() {
    assert_eq!(World::new().bounds(), None);
}

#[test]
fn set_tile_makes_the_tile_exactly_what_was_asked() {
    let mut world = World::new();
    let tree_on_grass = Tile { terrain: Some(Terrain::Grass), prop: Some(PropKind::Tree), marker: None, spawn: None, person: None };
    world.set_tile(2, 2, tree_on_grass);
    assert_eq!(world.tile(2, 2), tree_on_grass);
    // New ground under the same prop, then another prop, then none.
    for prop in [Some(PropKind::Tree), Some(PropKind::Rock), None] {
        let tile = Tile { terrain: Some(Terrain::Sand), prop, marker: None, spawn: None, person: None };
        world.set_tile(2, 2, tile);
        assert_eq!(world.tile(2, 2), tile);
    }
    // No ground: nothing can stand there.
    world.set_tile(2, 2, Tile { terrain: None, prop: Some(PropKind::Tree), marker: Some(Marker::PlayerStart), spawn: None, person: None });
    assert_eq!(world.tile(2, 2), Tile::default());
}

#[test]
fn every_change_marks_the_world_dirty_and_looking_does_not() {
    let mut world = World::new();
    assert!(!world.is_dirty());
    let _ = (world.terrain(0, 0), world.tile(0, 0), world.prop_at(0, 0), world.bounds());
    world.clear_tile(0, 0);
    assert!(!world.is_dirty(), "clearing what was never there changes nothing");
    world.set_terrain(0, 0, Terrain::Grass);
    assert!(world.is_dirty());
    let changes: [fn(&mut World); 3] = [
        |w| {
            w.place_prop(PropKind::Tree, 0, 0);
        },
        |w| {
            w.remove_prop(0, 0);
        },
        |w| w.clear_tile(0, 0),
    ];
    for change in changes {
        world.dirty.clear();
        change(&mut world);
        assert!(world.is_dirty());
    }
}

#[test]
fn a_marker_is_in_one_place_at_a_time_and_only_on_ground() {
    let mut world = World::new();
    assert!(!world.set_marker(Marker::PlayerStart, 3, 3), "no ground");
    world.set_terrain(3, 3, Terrain::Grass);
    world.set_terrain(50, -7, Terrain::Grass);
    assert!(world.set_marker(Marker::PlayerStart, 3, 3));
    assert_eq!(world.marker(Marker::PlayerStart), Some((3, 3)));
    assert_eq!(world.marker_at(3, 3), Some(Marker::PlayerStart));
    // Put elsewhere (another chunk), it leaves where it was.
    assert!(world.set_marker(Marker::PlayerStart, 50, -7));
    assert_eq!(world.marker(Marker::PlayerStart), Some((50, -7)));
    assert_eq!(world.marker_at(3, 3), None);
    // It goes with its ground.
    world.clear_tile(50, -7);
    assert_eq!(world.marker(Marker::PlayerStart), None);
}

#[test]
fn set_tile_carries_the_marker_too() {
    let mut world = World::new();
    world.set_terrain(1, 1, Terrain::Grass);
    let marked = Tile { terrain: Some(Terrain::Dirt), prop: None, marker: Some(Marker::PlayerStart), spawn: None, person: None };
    world.set_tile(8, 8, marked);
    assert_eq!(world.tile(8, 8), marked);
    world.set_tile(1, 1, Tile { terrain: Some(Terrain::Grass), prop: None, marker: Some(Marker::PlayerStart), spawn: None, person: None });
    assert_eq!(world.tile(8, 8).marker, None, "the marker moved");
    world.set_tile(1, 1, Tile { terrain: Some(Terrain::Grass), prop: None, marker: None, spawn: None, person: None });
    assert_eq!(world.marker(Marker::PlayerStart), None);
}

#[test]
fn water_trees_rocks_and_the_void_cannot_be_walked_on() {
    let mut world = World::new();
    for (x, terrain) in [(0, Terrain::Grass), (1, Terrain::Dirt), (2, Terrain::Sand), (3, Terrain::Water)] {
        world.set_terrain(x, 0, terrain);
    }
    assert!(world.walkable(0, 0) && world.walkable(1, 0) && world.walkable(2, 0));
    assert!(!world.walkable(3, 0), "water");
    assert!(!world.walkable(4, 0), "void");
    for (kind, blocks) in [(PropKind::Tree, true), (PropKind::Rock, true), (PropKind::HerbPurple, false), (PropKind::HerbCyan, false)] {
        world.remove_prop(0, 0);
        world.place_prop(kind, 0, 0);
        assert_eq!(world.walkable(0, 0), !blocks, "{kind:?}");
    }
}

#[test]
fn prop_names_round_trip() {
    for kind in PropKind::ALL {
        assert_eq!(PropKind::from_name(kind.name()), Some(kind));
    }
    assert_eq!(PropKind::from_name("dragon"), None);
}

#[test]
fn a_monsters_home_is_on_ground_one_to_a_tile_and_goes_with_it() {
    use crate::monsters::MonsterKind::Goblin;
    let mut world = World::new();
    assert!(!world.set_spawn(2, 2, Some(Goblin)), "no ground");
    for (x, y) in [(2, 2), (40, 3), (5, 1)] {
        world.set_terrain(x, y, Terrain::Grass);
        assert!(world.set_spawn(x, y, Some(Goblin)));
    }
    assert!(world.set_spawn(2, 2, Some(Goblin)), "again changes nothing");
    assert_eq!(world.spawns(), vec![(Goblin, 5, 1), (Goblin, 2, 2), (Goblin, 40, 3)], "row by row");
    assert_eq!(world.spawn_at(2, 2), Some(Goblin));
    assert!(world.set_spawn(2, 2, None));
    world.clear_tile(40, 3);
    assert_eq!(world.spawns(), vec![(Goblin, 5, 1)]);
    // And set_tile carries it like everything else about a tile.
    let tile = Tile { terrain: Some(Terrain::Dirt), prop: None, marker: None, spawn: Some(Goblin), person: None };
    world.set_tile(9, 9, tile);
    assert_eq!(world.tile(9, 9), tile);
}

#[test]
fn a_persons_home_is_on_ground_one_to_a_tile_and_goes_with_it() {
    use crate::people::PersonKind::{Guard, Smith};
    let mut world = World::new();
    assert!(!world.set_person(2, 2, Some(Smith)), "no ground");
    for (x, y) in [(2, 2), (40, 3)] {
        world.set_terrain(x, y, Terrain::Grass);
    }
    assert!(world.set_person(2, 2, Some(Smith)) && world.set_person(40, 3, Some(Guard)));
    assert!(world.set_person(2, 2, Some(Guard)), "someone else moves in");
    assert_eq!(world.people(), vec![(Guard, 2, 2), (Guard, 40, 3)], "row by row, one to a tile");
    assert_eq!((world.person_at(2, 2), world.person_at(3, 2)), (Some(Guard), None));
    // The ground going takes them with it, and a chunk left with only
    // a person in it is not empty.
    world.clear_tile(40, 3);
    assert_eq!(world.people(), vec![(Guard, 2, 2)]);
    let tile = Tile { terrain: Some(Terrain::Dirt), prop: None, marker: None, spawn: None, person: Some(Smith) };
    world.set_tile(9, 9, tile);
    assert_eq!(world.tile(9, 9), tile);
    assert!(world.set_person(2, 2, None) && world.set_person(9, 9, None));
    assert_eq!(world.people(), vec![]);
}
