//! The dice. One generator, seeded, so the same game played the same
//! way rolls the same: a test can say what will happen.

/// SplitMix64.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..1`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// `true` that share of the time: never for 0 or less, always for 1
    /// or more.
    pub fn chance(&mut self, share: f64) -> bool {
        self.unit() < share
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_rolls_the_same_and_another_does_not() {
        let rolls = |seed| {
            let mut rng = Rng::new(seed);
            (0..8).map(|_| rng.next_u64()).collect::<Vec<_>>()
        };
        assert_eq!(rolls(7), rolls(7));
        assert_ne!(rolls(7), rolls(8));
    }

    #[test]
    fn chances_come_up_about_as_often_as_they_say() {
        let mut rng = Rng::new(42);
        let hits = (0..10_000).filter(|_| rng.chance(0.3)).count();
        assert!((2800..3200).contains(&hits), "{hits} of 10000 at 30%");
        assert!((0..100).all(|_| !rng.chance(0.0) && rng.chance(1.0)));
        assert!((0..1000).all(|_| (0.0..1.0).contains(&rng.unit())));
    }
}
