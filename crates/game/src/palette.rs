//! The colours the world is drawn in: the Earthy palette from DESIGN.md.
//! They are the art's own sRGB numbers, which is what the world pass
//! works in.

use sim::Terrain;

/// A colour as the art has it: `0xRRGGBB` to sRGB numbers in `0..=1`.
pub const fn rgb(hex: u32) -> [f32; 3] {
    [((hex >> 16) & 0xff) as f32 / 255.0, ((hex >> 8) & 0xff) as f32 / 255.0, (hex & 0xff) as f32 / 255.0]
}

/// Where there is no ground at all.
pub const VOID: [f32; 3] = rgb(0x271a1c);

/// How one kind of ground is painted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainStyle {
    pub fill: [f32; 3],
    /// The band just inside its edge, where it lies over other ground.
    pub rim: [f32; 3],
    /// How wide that band is, in tiles. 0 for none.
    pub rim_width: f32,
}

pub const fn terrain_style(terrain: Terrain) -> TerrainStyle {
    match terrain {
        Terrain::Grass => TerrainStyle { fill: rgb(0x73a14c), rim: rgb(0x4f7e35), rim_width: 0.06 },
        Terrain::Dirt => TerrainStyle { fill: rgb(0xbd9d6b), rim: rgb(0x947340), rim_width: 0.08 },
        Terrain::Sand => TerrainStyle { fill: rgb(0xccb98a), rim: rgb(0xccb98a), rim_width: 0.0 },
        Terrain::Water => TerrainStyle { fill: rgb(0x5e96b8), rim: rgb(0x457295), rim_width: 0.06 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_splits_into_its_three_channels() {
        assert_eq!(rgb(0xff8000), [1.0, 128.0 / 255.0, 0.0]);
        assert_eq!(rgb(0x000000), [0.0; 3]);
    }

    #[test]
    fn every_terrain_has_a_style_and_no_rim_is_wider_than_a_corner() {
        for t in Terrain::ALL {
            let style = terrain_style(t);
            // A rim past the corner radius would swallow a lone tile.
            assert!((0.0..0.5).contains(&style.rim_width), "{t:?}");
        }
    }
}
