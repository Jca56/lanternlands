//! What the ground is made of.

/// The ground of one tile. The order is the stacking order where two
/// kinds meet: a later one lies over an earlier one, so a path sits on
/// the grass and water sits in its sand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Terrain {
    #[default]
    Grass = 0,
    Dirt = 1,
    Sand = 2,
    Water = 3,
}

impl Terrain {
    pub const COUNT: usize = 4;
    pub const ALL: [Terrain; Terrain::COUNT] = [Terrain::Grass, Terrain::Dirt, Terrain::Sand, Terrain::Water];

    /// Where it stacks: 0 is the bottom.
    pub const fn rank(self) -> u8 {
        self as u8
    }

    pub const fn from_rank(rank: u8) -> Option<Terrain> {
        match rank {
            0 => Some(Terrain::Grass),
            1 => Some(Terrain::Dirt),
            2 => Some(Terrain::Sand),
            3 => Some(Terrain::Water),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_round_trip_and_follow_the_order() {
        for (i, t) in Terrain::ALL.iter().enumerate() {
            assert_eq!(t.rank() as usize, i);
            assert_eq!(Terrain::from_rank(t.rank()), Some(*t));
        }
        assert_eq!(Terrain::from_rank(Terrain::COUNT as u8), None);
        assert!(Terrain::Grass < Terrain::Dirt && Terrain::Sand < Terrain::Water);
    }
}
