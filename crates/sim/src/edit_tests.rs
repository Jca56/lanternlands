//! Tests for the edit tools and their history.

use super::*;
use Terrain::{Dirt, Grass, Sand, Water};

/// A 10 by 10 field of grass.
fn field() -> World {
    let mut world = World::new();
    for y in 0..10 {
        for x in 0..10 {
            world.set_terrain(x, y, Grass);
        }
    }
    world
}

fn snapshot(world: &World) -> Vec<Tile> {
    (-2..12).flat_map(|y| (-2..12).map(move |x| (x, y))).map(|(x, y)| world.tile(x, y)).collect()
}

fn count(world: &World, terrain: Terrain) -> usize {
    snapshot(world).iter().filter(|t| t.terrain == Some(terrain)).count()
}

#[test]
fn brushes_are_round_symmetrical_and_the_size_they_say() {
    assert_eq!(brush((4.7, 2.1), 1), vec![(4, 2)]);
    // Two across sits on the nearest corner: the four tiles around it.
    let mut two = brush((4.7, 2.1), 2);
    two.sort();
    assert_eq!(two, vec![(4, 1), (4, 2), (5, 1), (5, 2)]);
    for size in 1..=9 {
        let tiles = brush((0.5, 0.5), size);
        let (xs, ys): (Vec<i32>, Vec<i32>) = tiles.iter().copied().unzip();
        let across = |v: &[i32]| v.iter().max().unwrap() - v.iter().min().unwrap() + 1;
        assert_eq!((across(&xs), across(&ys)), (size as i32, size as i32), "size {size}");
        // Mirrored left to right and top to bottom about its middle.
        let (sx, sy) = (xs.iter().min().unwrap() + xs.iter().max().unwrap(), ys.iter().min().unwrap() + ys.iter().max().unwrap());
        assert!(tiles.iter().all(|&(x, y)| tiles.contains(&(sx - x, y)) && tiles.contains(&(x, sy - y))), "size {size}");
        // Round: once it is big enough to have corners, they are cut.
        let square = (size * size) as usize;
        assert!(if size >= 4 { tiles.len() < square } else { tiles.len() == square }, "size {size}: {} tiles", tiles.len());
    }
}

#[test]
fn a_stroke_is_one_step_and_undo_puts_everything_back() {
    let mut world = field();
    let before = snapshot(&world);
    let mut history = History::new();
    history.begin();
    paint_line(&mut history, &mut world, (1.5, 1.5), (8.5, 6.5), 3, Some(Dirt));
    assert!(history.commit());
    let after = snapshot(&world);
    assert!(count(&world, Dirt) > 20);

    assert!(history.undo(&mut world));
    assert_eq!(snapshot(&world), before);
    assert!(!history.undo(&mut world), "it was one step");
    assert!(history.redo(&mut world));
    assert_eq!(snapshot(&world), after);
    assert!(!history.redo(&mut world));
}

#[test]
fn a_fast_drag_leaves_no_gaps() {
    let mut world = field();
    let mut history = History::new();
    paint_line(&mut history, &mut world, (0.5, 0.5), (9.5, 9.5), 1, Some(Dirt));
    // Every row and every column the drag crossed has dirt in it.
    for i in 0..10 {
        assert!((0..10).any(|x| world.terrain(x, i) == Some(Dirt)), "row {i}");
        assert!((0..10).any(|y| world.terrain(i, y) == Some(Dirt)), "column {i}");
    }
}

#[test]
fn painting_keeps_what_stands_and_erasing_takes_it() {
    let mut world = field();
    world.place_prop(PropKind::Tree, 5, 5);
    let mut history = History::new();
    paint(&mut history, &mut world, (5.5, 5.5), 3, Some(Sand));
    assert_eq!(world.tile(5, 5), Tile { terrain: Some(Sand), prop: Some(PropKind::Tree), marker: None, spawn: None });
    paint(&mut history, &mut world, (5.5, 5.5), 1, None);
    assert_eq!(world.tile(5, 5), Tile::default());
    // Undo brings the tree back with its ground.
    history.undo(&mut world);
    assert_eq!(world.tile(5, 5), Tile { terrain: Some(Grass), prop: Some(PropKind::Tree), marker: None, spawn: None });
}

#[test]
fn painting_past_the_edge_makes_new_ground() {
    let mut world = field();
    let mut history = History::new();
    paint(&mut history, &mut world, (-0.5, -0.5), 1, Some(Water));
    assert_eq!(world.terrain(-1, -1), Some(Water));
    history.undo(&mut world);
    assert_eq!(world.terrain(-1, -1), None);
}

#[test]
fn a_step_that_changes_nothing_is_not_a_step() {
    let mut world = field();
    let mut history = History::new();
    history.begin();
    paint(&mut history, &mut world, (3.5, 3.5), 3, Some(Grass));
    assert!(!history.commit());
    // Changed and changed back within one step: nothing either.
    history.begin();
    paint(&mut history, &mut world, (3.5, 3.5), 1, Some(Dirt));
    paint(&mut history, &mut world, (3.5, 3.5), 1, Some(Grass));
    assert!(!history.commit());
    assert!(!history.can_undo());
}

#[test]
fn a_new_edit_forgets_what_was_undone() {
    let mut world = field();
    let mut history = History::new();
    for terrain in [Dirt, Sand] {
        history.begin();
        paint(&mut history, &mut world, (2.5, 2.5), 1, Some(terrain));
        history.commit();
    }
    history.undo(&mut world);
    assert!(history.can_redo());
    history.begin();
    paint(&mut history, &mut world, (7.5, 7.5), 1, Some(Water));
    history.commit();
    assert!(!history.can_redo());
    assert_eq!(world.terrain(2, 2), Some(Dirt));
}

#[test]
fn the_history_forgets_its_oldest_steps_past_the_limit() {
    let mut world = field();
    let mut history = History::new();
    for i in 0..HISTORY_LIMIT + 10 {
        history.begin();
        paint(&mut history, &mut world, (0.5, 0.5), 1, Some(if i % 2 == 0 { Dirt } else { Sand }));
        history.commit();
    }
    let mut steps = 0;
    while history.undo(&mut world) {
        steps += 1;
    }
    assert_eq!(steps, HISTORY_LIMIT);
}

#[test]
fn fill_floods_one_joined_stretch_and_stops_at_other_ground() {
    let mut world = field();
    // A wall of dirt down column 4 splits the field in two.
    for y in 0..10 {
        world.set_terrain(4, y, Dirt);
    }
    world.place_prop(PropKind::Rock, 1, 1);
    let mut history = History::new();
    history.begin();
    assert_eq!(fill(&mut history, &mut world, 0, 0, Water), 40);
    history.commit();
    assert_eq!(count(&world, Water), 40);
    assert_eq!(count(&world, Grass), 50, "the other side of the wall is untouched");
    assert_eq!(world.prop_at(1, 1).map(|p| p.kind), Some(PropKind::Rock), "what stood there still stands");
    // Tiles touching only at a corner are not joined.
    world.set_terrain(4, 0, Grass);
    world.set_terrain(5, 0, Dirt);
    assert_eq!(fill(&mut history, &mut world, 4, 0, Sand), 1);
    // No ground, or the same ground: nothing to do.
    assert_eq!(fill(&mut history, &mut world, -5, -5, Sand), 0);
    assert_eq!(fill(&mut history, &mut world, 0, 0, Water), 0);
    // And the whole flood is one step.
    history.commit();
    history.undo(&mut world);
    history.undo(&mut world);
    assert_eq!(count(&world, Water), 0);
}

#[test]
fn placing_replaces_and_removing_removes_each_as_a_step() {
    let mut world = field();
    let mut history = History::new();
    history.begin();
    assert!(place(&mut history, &mut world, 3, 3, PropKind::Tree));
    assert!(place(&mut history, &mut world, 3, 3, PropKind::Rock));
    assert!(!place(&mut history, &mut world, -9, -9, PropKind::Tree), "no ground there");
    history.commit();
    assert_eq!(world.prop_at(3, 3).map(|p| p.kind), Some(PropKind::Rock));
    history.begin();
    assert!(remove(&mut history, &mut world, 3, 3));
    assert!(!remove(&mut history, &mut world, 3, 3));
    history.commit();
    assert_eq!(world.prop_at(3, 3), None);
    history.undo(&mut world);
    assert_eq!(world.prop_at(3, 3).map(|p| p.kind), Some(PropKind::Rock));
    history.undo(&mut world);
    assert_eq!(world.prop_at(3, 3), None);
}

#[test]
fn moving_a_prop_takes_it_to_free_ground_and_nowhere_else() {
    let mut world = field();
    world.place_prop(PropKind::Tree, 2, 2);
    world.place_prop(PropKind::Rock, 6, 6);
    let mut history = History::new();
    history.begin();
    assert!(!move_prop(&mut history, &mut world, (2, 2), (2, 2)), "onto itself");
    assert!(!move_prop(&mut history, &mut world, (2, 2), (6, 6)), "onto another prop");
    assert!(!move_prop(&mut history, &mut world, (2, 2), (-3, 2)), "onto no ground");
    assert!(!move_prop(&mut history, &mut world, (4, 4), (5, 5)), "nothing stands there to move");
    assert!(!history.commit(), "none of that changed anything");
    assert_eq!(world.prop_at(2, 2).map(|p| p.kind), Some(PropKind::Tree));

    history.begin();
    assert!(move_prop(&mut history, &mut world, (2, 2), (8, 3)));
    history.commit();
    assert_eq!(world.prop_at(2, 2), None);
    assert_eq!(world.prop_at(8, 3).map(|p| p.kind), Some(PropKind::Tree));
    assert_eq!(world.props_in(0, 0, 10, 10).count(), 2, "nothing was made or lost");
}

#[test]
fn a_move_in_many_hops_is_one_step_back_to_where_it_started() {
    let mut world = field();
    // The ground differs along the way and must not be disturbed.
    world.set_terrain(3, 2, Sand);
    world.place_prop(PropKind::Tree, 2, 2);
    let before = snapshot(&world);
    let mut history = History::new();
    history.begin();
    for hop in [((2, 2), (3, 2)), ((3, 2), (4, 2)), ((4, 2), (4, 3))] {
        assert!(move_prop(&mut history, &mut world, hop.0, hop.1));
    }
    assert!(history.commit());
    assert_eq!(world.prop_at(4, 3).map(|p| p.kind), Some(PropKind::Tree));
    assert_eq!(world.terrain(3, 2), Some(Sand));
    assert!(history.undo(&mut world));
    assert_eq!(snapshot(&world), before);
    assert!(!history.can_undo(), "the hops were one step");
    assert!(history.redo(&mut world));
    assert_eq!(world.prop_at(4, 3).map(|p| p.kind), Some(PropKind::Tree));
    assert_eq!(world.prop_at(2, 2), None);
}

#[test]
fn a_prop_moved_out_and_back_is_no_step_at_all() {
    let mut world = field();
    world.place_prop(PropKind::Rock, 5, 5);
    let mut history = History::new();
    history.begin();
    assert!(move_prop(&mut history, &mut world, (5, 5), (6, 5)));
    assert!(move_prop(&mut history, &mut world, (6, 5), (5, 5)));
    assert!(!history.commit());
}

#[test]
fn a_prop_moves_from_one_chunk_to_another() {
    let mut world = World::new();
    for x in 30..34 {
        world.set_terrain(x, 0, Grass);
    }
    world.place_prop(PropKind::HerbCyan, 31, 0);
    let mut history = History::new();
    assert!(move_prop(&mut history, &mut world, (31, 0), (33, 0)));
    assert_eq!(world.prop_at(33, 0).map(|p| p.kind), Some(PropKind::HerbCyan));
    assert_eq!(world.props_in(0, 0, 64, 32).count(), 1);
}

#[test]
fn undo_in_the_middle_of_a_stroke_takes_the_stroke_so_far() {
    let mut world = field();
    let before = snapshot(&world);
    let mut history = History::new();
    history.begin();
    paint(&mut history, &mut world, (2.5, 2.5), 3, Some(Dirt));
    assert!(history.undo(&mut world));
    assert_eq!(snapshot(&world), before);
}

#[test]
fn setting_the_start_moves_it_and_undo_puts_it_back_where_it_was() {
    let mut world = field();
    let mut history = History::new();
    history.begin();
    assert!(set_marker(&mut history, &mut world, Marker::PlayerStart, 2, 2));
    history.commit();
    history.begin();
    assert!(set_marker(&mut history, &mut world, Marker::PlayerStart, 7, 5));
    assert!(!set_marker(&mut history, &mut world, Marker::PlayerStart, -4, -4), "no ground there");
    history.commit();
    assert_eq!(world.marker(Marker::PlayerStart), Some((7, 5)));
    assert_eq!(world.marker_at(2, 2), None);

    history.undo(&mut world);
    assert_eq!(world.marker(Marker::PlayerStart), Some((2, 2)));
    history.undo(&mut world);
    assert_eq!(world.marker(Marker::PlayerStart), None);
    history.redo(&mut world);
    history.redo(&mut world);
    assert_eq!(world.marker(Marker::PlayerStart), Some((7, 5)));
}

#[test]
fn the_tools_leave_a_marker_where_it_is_and_the_eraser_takes_it() {
    let mut world = field();
    let mut history = History::new();
    set_marker(&mut history, &mut world, Marker::PlayerStart, 4, 4);
    paint(&mut history, &mut world, (4.5, 4.5), 3, Some(Sand));
    place(&mut history, &mut world, 4, 4, PropKind::HerbCyan);
    fill(&mut history, &mut world, 4, 4, Dirt);
    remove(&mut history, &mut world, 4, 4);
    assert_eq!(world.tile(4, 4), Tile { terrain: Some(Dirt), prop: None, marker: Some(Marker::PlayerStart), spawn: None });
    paint(&mut history, &mut world, (4.5, 4.5), 1, None);
    assert_eq!(world.marker(Marker::PlayerStart), None);
    history.undo(&mut world);
    assert_eq!(world.marker(Marker::PlayerStart), None, "the whole run was one step");
}

#[test]
fn a_monsters_home_is_set_and_taken_away_each_as_a_step() {
    use crate::monsters::MonsterKind::Goblin;
    let mut world = field();
    let mut history = History::new();
    history.begin();
    assert!(set_spawn(&mut history, &mut world, 3, 3, Some(Goblin)));
    assert!(!set_spawn(&mut history, &mut world, -5, 3, Some(Goblin)), "no ground there");
    history.commit();
    // Painting under it leaves it; erasing the ground takes it.
    paint(&mut history, &mut world, (3.5, 3.5), 1, Some(Sand));
    assert_eq!(world.spawn_at(3, 3), Some(Goblin));
    history.begin();
    assert!(set_spawn(&mut history, &mut world, 3, 3, None));
    history.commit();
    assert_eq!(world.spawns(), vec![]);
    history.undo(&mut world);
    assert_eq!(world.spawns(), vec![(Goblin, 3, 3)]);
}
