//! How someone on foot looks: which way they face, and which drawing of
//! their walk shows at a given moment of a tick.

use crate::assets::{SpriteId, SpriteSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Facing {
    #[default]
    Down,
    Up,
    Left,
    Right,
}

impl Facing {
    /// The way a step by `(dx, dy)` faces: sideways on a diagonal, since
    /// that is the way the side views tell apart. `None` for no step.
    pub fn of_step(dx: i32, dy: i32) -> Option<Facing> {
        match (dx.signum(), dy.signum()) {
            (0, 0) => None,
            (1, _) => Some(Facing::Right),
            (-1, _) => Some(Facing::Left),
            (_, 1) => Some(Facing::Down),
            _ => Some(Facing::Up),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose {
    Idle,
    /// One foot forward.
    StepA,
    /// Feet together, body at its highest.
    Pass,
    /// The other foot forward.
    StepB,
    /// Arms drawn back to strike.
    WorkA,
    /// The strike.
    WorkB,
}

impl Pose {
    pub const ALL: [Pose; 6] = [Pose::Idle, Pose::StepA, Pose::Pass, Pose::StepB, Pose::WorkA, Pose::WorkB];

    /// The pose `into` a tick for someone who swings: the strike as the
    /// tick it `struck` on begins, and the drawing back as the tick
    /// before it (when one is `due`) ends, so one swing is one drawing
    /// back and one strike. `None` the rest of the time.
    pub fn swing(struck: bool, due: bool, into: f64) -> Option<Pose> {
        if struck && into < 0.5 {
            Some(Pose::WorkB)
        } else if due && into >= 0.5 {
            Some(Pose::WorkA)
        } else {
            None
        }
    }

    /// The pose `into` a tick (0 at its start, 1 at its end) for someone
    /// who has taken `steps` steps and did or did not move this tick. A
    /// step is a stride then a pass, and strides take turns.
    pub fn at(moved: bool, steps: u64, into: f64) -> Pose {
        if !moved {
            Pose::Idle
        } else if into >= 0.5 {
            Pose::Pass
        } else if steps.is_multiple_of(2) {
            Pose::StepA
        } else {
            Pose::StepB
        }
    }
}

/// Someone's drawings: down, up and side, each idle, walking (step,
/// pass, step) and at work (drawn back, striking).
#[derive(Clone, Copy, Debug, Default)]
pub struct WalkSprites([[Option<SpriteId>; 6]; 3]);

const ROWS: [&str; 3] = ["down", "up", "side"];
const COLUMNS: [&str; 6] = ["idle", "step_a", "pass", "step_b", "work_a", "work_b"];

impl WalkSprites {
    /// The sprites called `name_row_column`, as a walk sheet's grid
    /// names them.
    pub fn find(set: &SpriteSet, name: &str) -> WalkSprites {
        WalkSprites(ROWS.map(|row| COLUMNS.map(|column| set.id(&format!("{name}_{row}_{column}")))))
    }

    /// The names of those it looked for and did not find.
    pub fn missing(&self, name: &str) -> Vec<String> {
        let cells = (0..ROWS.len()).flat_map(|r| (0..COLUMNS.len()).map(move |c| (r, c)));
        cells.filter(|&(r, c)| self.0[r][c].is_none()).map(|(r, c)| format!("{name}_{}_{}", ROWS[r], COLUMNS[c])).collect()
    }

    /// The drawing for a facing and a pose, and whether to mirror it:
    /// the side view is drawn facing right.
    pub fn sprite(&self, facing: Facing, pose: Pose) -> Option<(SpriteId, bool)> {
        let (row, flip) = match facing {
            Facing::Down => (0, false),
            Facing::Up => (1, false),
            Facing::Right => (2, false),
            Facing::Left => (2, true),
        };
        let column = Pose::ALL.iter().position(|p| *p == pose).expect("every pose is listed");
        self.0[row][column].map(|id| (id, flip))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_faces_the_way_it_goes_and_sideways_on_a_diagonal() {
        assert_eq!(Facing::of_step(0, 0), None);
        assert_eq!(Facing::of_step(0, 1), Some(Facing::Down));
        assert_eq!(Facing::of_step(0, -1), Some(Facing::Up));
        assert_eq!(Facing::of_step(1, 0), Some(Facing::Right));
        assert_eq!(Facing::of_step(-1, 0), Some(Facing::Left));
        for dy in [-1, 1] {
            assert_eq!(Facing::of_step(1, dy), Some(Facing::Right));
            assert_eq!(Facing::of_step(-1, dy), Some(Facing::Left));
        }
    }

    #[test]
    fn a_tick_on_the_move_is_a_stride_then_a_pass_and_strides_take_turns() {
        assert_eq!(Pose::at(false, 7, 0.3), Pose::Idle);
        assert_eq!((Pose::at(true, 4, 0.0), Pose::at(true, 4, 0.49), Pose::at(true, 4, 0.5), Pose::at(true, 4, 0.99)), (Pose::StepA, Pose::StepA, Pose::Pass, Pose::Pass));
        assert_eq!((Pose::at(true, 5, 0.2), Pose::at(true, 5, 0.8)), (Pose::StepB, Pose::Pass));
    }

    #[test]
    fn a_swing_is_one_drawing_back_and_one_strike_across_two_ticks() {
        // A swing is the end of one tick and the start of the next.
        assert_eq!((Pose::swing(false, true, 0.49), Pose::swing(false, true, 0.5), Pose::swing(false, true, 1.0)), (None, Some(Pose::WorkA), Some(Pose::WorkA)));
        assert_eq!((Pose::swing(true, false, 0.0), Pose::swing(true, false, 0.49), Pose::swing(true, false, 0.5)), (Some(Pose::WorkB), Some(Pose::WorkB), None));
        assert_eq!(Pose::swing(true, true, 0.2), Some(Pose::WorkB), "struck, and due again: strike, then draw back");
        assert_eq!(Pose::swing(true, true, 0.8), Some(Pose::WorkA));
        assert!((0..=10).all(|i| Pose::swing(false, false, i as f64 / 10.0).is_none()));
    }

    #[test]
    fn the_shipped_sheet_has_every_drawing_and_left_is_right_mirrored() {
        let set = SpriteSet::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")).expect("assets load");
        let walk = WalkSprites::find(&set, "player");
        assert_eq!(walk.missing("player"), Vec::<String>::new());
        let mut seen = std::collections::HashSet::new();
        for facing in [Facing::Down, Facing::Up, Facing::Right] {
            for pose in Pose::ALL {
                let (id, flip) = walk.sprite(facing, pose).expect("drawn");
                assert!(!flip && seen.insert(id), "{facing:?} {pose:?} is its own drawing, unmirrored");
            }
        }
        for pose in Pose::ALL {
            let (right, _) = walk.sprite(Facing::Right, pose).unwrap();
            assert_eq!(walk.sprite(Facing::Left, pose), Some((right, true)));
        }
        assert_eq!(WalkSprites::find(&set, "nobody").missing("nobody").len(), 18);
    }
}
