//! A small made-up map to look at until the map editor exists: grass, a
//! winding path with a clearing, two ponds in their sand, and trees,
//! rocks and herbs scattered about. Not game content; it goes away when
//! real maps can be made.

use crate::noise::unit;
use crate::terrain::Terrain;
use crate::world::{PropKind, World};

pub const WIDTH: i32 = 48;
pub const HEIGHT: i32 = 32;

/// The path's corners, in tiles.
const PATH: [(f64, f64); 6] = [(4.0, 28.0), (10.0, 22.0), (18.0, 20.0), (24.0, 14.0), (34.0, 12.0), (44.0, 6.0)];
const BRANCH: [(f64, f64); 2] = [(18.0, 20.0), (20.5, 29.0)];
const PATH_HALF_WIDTH: f64 = 0.9;
const CLEARING: (f64, f64, f64) = (24.0, 14.0, 2.6);
/// Ponds: centre x, centre y, radius x, radius y.
const PONDS: [(f64, f64, f64, f64); 2] = [(33.0, 22.0, 5.5, 3.5), (10.0, 8.0, 2.6, 2.0)];
const SAND_RING: f64 = 1.3;
/// Where trees crowd together, and where rocks do: centre and radius.
const WOODS: [(f64, f64, f64); 2] = [(41.0, 27.0, 6.5), (5.0, 17.0, 4.5)];
const QUARRY: (f64, f64, f64) = (42.0, 11.0, 4.0);

/// The same map every time.
pub fn build() -> World {
    let mut world = World::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            world.set_terrain(x, y, ground(x, y));
        }
    }
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if let Some(kind) = prop(&world, x, y) {
                world.place_prop(kind, x, y);
            }
        }
    }
    world
}

fn ground(x: i32, y: i32) -> Terrain {
    let (cx, cy) = (x as f64 + 0.5, y as f64 + 0.5);
    for &(px, py, rx, ry) in &PONDS {
        if ellipse(cx, cy, px, py, rx, ry) <= 1.0 {
            return Terrain::Water;
        }
    }
    for &(px, py, rx, ry) in &PONDS {
        if ellipse(cx, cy, px, py, rx + SAND_RING, ry + SAND_RING) <= 1.0 {
            return Terrain::Sand;
        }
    }
    let on_path = PATH.windows(2).chain(BRANCH.windows(2)).any(|s| segment_distance((cx, cy), s[0], s[1]) <= PATH_HALF_WIDTH);
    let in_clearing = (cx - CLEARING.0).hypot(cy - CLEARING.1) <= CLEARING.2;
    if on_path || in_clearing { Terrain::Dirt } else { Terrain::Grass }
}

/// What, if anything, stands on a tile: only on grass with grass all
/// around it, so nothing crowds a path or a shore.
fn prop(world: &World, x: i32, y: i32) -> Option<PropKind> {
    let open = (-1..=1).all(|dy| (-1..=1).all(|dx| world.terrain(x + dx, y + dy).is_none_or(|t| t == Terrain::Grass)));
    if !open {
        return None;
    }
    let (cx, cy) = (x as f64 + 0.5, y as f64 + 0.5);
    let near = |&(px, py, r): &(f64, f64, f64)| (cx - px).hypot(cy - py) <= r;
    let tree = if WOODS.iter().any(near) { 0.22 } else { 0.03 };
    let rock = if near(&QUARRY) { 0.16 } else { 0.008 };
    let roll = unit(x, y, 1);
    if roll < tree {
        Some(PropKind::Tree)
    } else if roll < tree + rock {
        Some(PropKind::Rock)
    } else if roll < tree + rock + 0.012 {
        Some(PropKind::HerbPurple)
    } else if roll < tree + rock + 0.02 {
        Some(PropKind::HerbCyan)
    } else {
        None
    }
}

/// 1 on the ellipse's edge, less inside.
fn ellipse(x: f64, y: f64, cx: f64, cy: f64, rx: f64, ry: f64) -> f64 {
    ((x - cx) / rx).hypot((y - cy) / ry)
}

fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let along = ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy);
    let t = along.clamp(0.0, 1.0);
    (p.0 - (a.0 + dx * t)).hypot(p.1 - (a.1 + dy * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(world: &World) -> Vec<(Option<Terrain>, Option<PropKind>)> {
        (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y))).map(|(x, y)| (world.terrain(x, y), world.prop_at(x, y).map(|p| p.kind))).collect()
    }

    #[test]
    fn it_is_the_same_map_every_time() {
        assert_eq!(snapshot(&build()), snapshot(&build()));
    }

    #[test]
    fn every_tile_inside_has_ground_and_none_outside() {
        let world = build();
        assert!(snapshot(&world).iter().all(|(t, _)| t.is_some()));
        assert_eq!(world.terrain(-1, 0), None);
        assert_eq!(world.terrain(WIDTH, 0), None);
        assert_eq!(world.terrain(0, HEIGHT), None);
    }

    #[test]
    fn the_landmarks_are_where_they_were_put() {
        let world = build();
        assert_eq!(world.terrain(33, 22), Some(Terrain::Water), "big pond");
        assert_eq!(world.terrain(10, 8), Some(Terrain::Water), "small pond");
        assert_eq!(world.terrain(27, 22), Some(Terrain::Sand), "the big pond's shore");
        assert_eq!(world.terrain(24, 14), Some(Terrain::Dirt), "the clearing");
        assert_eq!(world.terrain(10, 22), Some(Terrain::Dirt), "a corner of the path");
        assert_eq!(world.terrain(0, 0), Some(Terrain::Grass));
    }

    #[test]
    fn water_always_has_sand_or_water_beside_it() {
        let world = build();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if world.terrain(x, y) != Some(Terrain::Water) {
                    continue;
                }
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let beside = world.terrain(x + dx, y + dy);
                    assert!(matches!(beside, Some(Terrain::Water | Terrain::Sand)), "({x}, {y}) has {beside:?} beside it");
                }
            }
        }
    }

    #[test]
    fn props_stand_on_open_grass_and_every_kind_shows_up() {
        let world = build();
        let props: Vec<_> = world.props_in(0, 0, WIDTH, HEIGHT).collect();
        for p in &props {
            assert_eq!(world.terrain(p.x, p.y), Some(Terrain::Grass), "{p:?}");
        }
        for kind in [PropKind::Tree, PropKind::Rock, PropKind::HerbPurple, PropKind::HerbCyan] {
            assert!(props.iter().any(|p| p.kind == kind), "no {kind:?} on the map");
        }
        let trees = props.iter().filter(|p| p.kind == PropKind::Tree).count();
        assert!((20..200).contains(&trees), "{trees} trees");
    }
}
