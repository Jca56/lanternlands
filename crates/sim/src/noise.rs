//! Numbers that look random but depend only on what they are asked
//! about, so the same tile gets the same answer every time and in any
//! order.

/// A number in `0..1` that depends only on the tile and `salt`. Ask with
/// a different `salt` for an unrelated number about the same tile.
pub fn unit(x: i32, y: i32, salt: u64) -> f64 {
    // SplitMix64's finalizer over the three packed together.
    let mut z = (x as u32 as u64) | ((y as u32 as u64) << 32);
    z = z.wrapping_add(salt.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_stays_in_range_and_spreads() {
        let rolls: Vec<f64> = (0..1000).map(|i| unit(i % 37, i / 37, 1)).collect();
        assert!(rolls.iter().all(|r| (0.0..1.0).contains(r)));
        let mean = rolls.iter().sum::<f64>() / rolls.len() as f64;
        assert!((0.45..0.55).contains(&mean), "mean {mean}");
    }

    #[test]
    fn a_tile_always_gets_the_same_answer_and_salts_differ() {
        assert_eq!(unit(12, -7, 3), unit(12, -7, 3));
        assert_ne!(unit(12, -7, 3), unit(12, -7, 4));
        assert_ne!(unit(12, -7, 3), unit(-7, 12, 3));
    }
}
