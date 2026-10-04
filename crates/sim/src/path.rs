//! Finding a way across the world on foot.
//!
//! A step goes to any of the eight tiles around, and each takes one
//! tick. A diagonal step needs both tiles beside it to be free as well,
//! so nobody squeezes between two rocks or cuts across a pond's corner.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::world::World;

/// How many tiles a search looks at before settling for the nearest it
/// found. Far more than any click on screen needs.
const SEARCH_LIMIT: usize = 20_000;

const STEPS: [(i32, i32); 8] = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)];

/// Whether one step from `from` by `(dx, dy)` can be taken.
pub fn can_step(world: &World, from: (i32, i32), (dx, dy): (i32, i32)) -> bool {
    let (x, y) = from;
    world.walkable(x + dx, y + dy) && (dx == 0 || dy == 0 || (world.walkable(x + dx, y) && world.walkable(x, y + dy)))
}

/// Steps between two tiles going any of eight ways: the larger of the
/// two distances.
fn steps_between(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

fn blocks_between(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

/// The tiles to step through, in order, to walk from `from` to `to`:
/// as few steps as can be, going diagonally first and then straight.
/// When `to` cannot be reached (it is water, a tree, cut off, or too
/// far to search) the way leads to the reachable tile nearest it
/// instead. Empty when there is nowhere better to go than `from`.
pub fn find(world: &World, from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    find_around(world, from, to, &|_| false)
}

/// The same, going round every tile that is `taken`: somebody stands
/// there, and a tile holds one body. `to` itself may be taken; the way
/// then ends on it, for whoever walks it to stop a step short.
pub fn find_around(world: &World, from: (i32, i32), to: (i32, i32), taken: &dyn Fn((i32, i32)) -> bool) -> Vec<(i32, i32)> {
    // A*: `steps` taken so far plus the fewest that could remain. Among
    // equals the one fewer blocks from the goal goes first, which is
    // what makes the way diagonal first, and the oldest after that, so
    // the same search always gives the same way.
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut steps: HashMap<(i32, i32), i32> = HashMap::from([(from, 0)]);
    let mut open = BinaryHeap::from([Reverse((steps_between(from, to), blocks_between(from, to), 0u32, from))]);
    let mut nearest = (steps_between(from, to), blocks_between(from, to), from);
    let (mut order, mut looked) = (0u32, 0usize);
    while let Some(Reverse((_, _, _, tile))) = open.pop() {
        if tile == to {
            nearest.2 = to;
            break;
        }
        looked += 1;
        if looked > SEARCH_LIMIT {
            break;
        }
        let so_far = steps[&tile];
        for step in STEPS {
            let next = (tile.0 + step.0, tile.1 + step.1);
            if !can_step(world, tile, step) || (next != to && taken(next)) || steps.get(&next).is_some_and(|&known| known <= so_far + 1) {
                continue;
            }
            steps.insert(next, so_far + 1);
            came_from.insert(next, tile);
            let (left, blocks) = (steps_between(next, to), blocks_between(next, to));
            if (left, blocks) < (nearest.0, nearest.1) {
                nearest = (left, blocks, next);
            }
            order += 1;
            open.push(Reverse((so_far + 1 + left, blocks, order, next)));
        }
    }
    let mut way = Vec::new();
    let mut tile = nearest.2;
    while tile != from {
        way.push(tile);
        tile = came_from[&tile];
    }
    way.reverse();
    way
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::Terrain;
    use crate::world::PropKind;

    /// A world from a picture: `.` grass, `~` water, `T` a tree on
    /// grass, space nothing. The top-left character is tile (0, 0).
    fn world(rows: &[&str]) -> World {
        let mut world = World::new();
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                let (x, y) = (x as i32, y as i32);
                match c {
                    '.' => world.set_terrain(x, y, Terrain::Grass),
                    '~' => world.set_terrain(x, y, Terrain::Water),
                    'T' => {
                        world.set_terrain(x, y, Terrain::Grass);
                        world.place_prop(PropKind::Tree, x, y);
                    }
                    _ => {}
                }
            }
        }
        world
    }

    /// Every step of a way is one legal step from the tile before.
    fn legal(world: &World, from: (i32, i32), way: &[(i32, i32)]) -> bool {
        let mut at = from;
        way.iter().all(|&next| {
            let step = (next.0 - at.0, next.1 - at.1);
            let ok = step.0.abs() <= 1 && step.1.abs() <= 1 && step != (0, 0) && can_step(world, at, step);
            at = next;
            ok
        })
    }

    #[test]
    fn open_ground_is_crossed_diagonally_first_then_straight() {
        let w = world(&["........", "........", "........", "........"]);
        assert_eq!(find(&w, (0, 0), (5, 2)), vec![(1, 1), (2, 2), (3, 2), (4, 2), (5, 2)]);
        assert_eq!(find(&w, (6, 3), (6, 0)), vec![(6, 2), (6, 1), (6, 0)]);
        assert_eq!(find(&w, (3, 3), (3, 3)), vec![]);
    }

    #[test]
    fn a_taken_tile_is_gone_round_and_a_taken_goal_is_still_led_to() {
        let w = world(&["........", "........", "........"]);
        let taken = |tile: (i32, i32)| tile == (3, 1) || tile == (6, 1);
        let way = find_around(&w, (0, 1), (5, 1), &taken);
        assert!(!way.contains(&(3, 1)), "{way:?}");
        assert_eq!((way.len(), way.last()), (5, Some(&(5, 1))));
        assert!(legal(&w, (0, 1), &way));
        // The goal itself may be taken: the way ends on it.
        let way = find_around(&w, (0, 1), (6, 1), &taken);
        assert_eq!(way.last(), Some(&(6, 1)));
        assert!(!way.contains(&(3, 1)));
        // With nothing taken it is the plain search.
        assert_eq!(find_around(&w, (0, 1), (5, 1), &|_| false), find(&w, (0, 1), (5, 1)));
    }

    #[test]
    fn a_way_takes_as_few_steps_as_the_distance_allows() {
        let w = world(&["..........", "..........", "..........", "..........", "..........", ".........."]);
        for (from, to) in [((0, 0), (9, 5)), ((9, 0), (0, 5)), ((4, 5), (4, 0)), ((0, 3), (9, 3)), ((7, 1), (2, 4))] {
            let way = find(&w, from, to);
            assert_eq!(way.len() as i32, steps_between(from, to), "{from:?} to {to:?}");
            assert_eq!(way.last(), Some(&to));
            assert!(legal(&w, from, &way));
        }
    }

    #[test]
    fn it_goes_round_what_is_in_the_way() {
        let w = world(&[
            ".......",
            "..TTT..",
            "..T~T..",
            "..TTT..",
            ".......",
        ]);
        let way = find(&w, (0, 2), (6, 2));
        assert_eq!(way.last(), Some(&(6, 2)));
        assert!(legal(&w, (0, 2), &way));
        assert!(way.iter().all(|&(x, y)| w.walkable(x, y)));
        // Six tiles across, and the block costs two more to get round.
        assert_eq!(way.len(), 8);
    }

    #[test]
    fn no_step_cuts_a_corner() {
        // The only gap in the wall is on a diagonal between two trees.
        let w = world(&[
            "..T..",
            ".T...",
            ".....",
        ]);
        assert!(!can_step(&w, (1, 0), (1, 1)), "between a tree and a tree");
        let w = world(&[
            ".T.",
            "...",
        ]);
        assert!(!can_step(&w, (0, 0), (1, 1)), "round the corner of one tree");
        assert!(can_step(&w, (0, 1), (1, 0)));
        // So the way round a single tree is three steps, not two.
        assert_eq!(find(&w, (0, 0), (2, 0)).len(), 4);
    }

    #[test]
    fn a_tile_that_cannot_be_stood_on_is_walked_up_to() {
        let w = world(&[
            ".......",
            "...~~~.",
            "...~~~.",
            "......T",
        ]);
        // Into the pond: to the shore right beside the tile asked for,
        // not merely the nearest bank.
        let way = find(&w, (0, 1), (4, 1));
        assert_eq!(way.last(), Some(&(4, 0)));
        // Into its far row, which only the west bank is beside.
        let way = find(&w, (0, 2), (3, 2));
        assert_eq!(way.last(), Some(&(2, 2)));
        // At a tree: to a tile beside it.
        let way = find(&w, (0, 3), (6, 3));
        assert_eq!(way.last(), Some(&(5, 3)));
        // Off the map: as near as the ground goes.
        let way = find(&w, (0, 0), (20, 0));
        assert_eq!(way.last(), Some(&(6, 0)));
    }

    #[test]
    fn somewhere_cut_off_is_approached_as_near_as_can_be() {
        let w = world(&[
            "...~...",
            "...~...",
            "...~...",
        ]);
        let way = find(&w, (0, 1), (6, 1));
        assert_eq!(way.last(), Some(&(2, 1)), "the bank facing it");
        assert!(legal(&w, (0, 1), &way));
        // Already as near as can be: nowhere to go.
        assert_eq!(find(&w, (2, 1), (6, 1)), vec![]);
    }

    #[test]
    fn the_same_search_gives_the_same_way() {
        let w = crate::testmap::build();
        let (from, to) = ((2, 2), (45, 29));
        let way = find(&w, from, to);
        assert_eq!(way, find(&w, from, to));
        assert!(legal(&w, from, &way));
        assert!(!way.is_empty());
    }

    #[test]
    fn a_search_across_the_whole_test_map_ends_at_or_beside_the_goal() {
        let w = crate::testmap::build();
        let walkable: Vec<(i32, i32)> = (0..crate::testmap::HEIGHT).flat_map(|y| (0..crate::testmap::WIDTH).map(move |x| (x, y))).filter(|&(x, y)| w.walkable(x, y)).collect();
        let from = walkable[0];
        for &to in walkable.iter().step_by(37) {
            let way = find(&w, from, to);
            assert!(legal(&w, from, &way), "{from:?} to {to:?}");
            // The test map's walkable ground is all in one piece.
            assert_eq!(way.last().copied().unwrap_or(from), to, "{from:?} to {to:?}");
            assert!(way.len() as i32 >= steps_between(from, to));
        }
    }
}
