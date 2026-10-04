//! Packs sprites into one sheet for the GPU, with its smaller copies.
//!
//! The sheet holds premultiplied colour in the pictures' own sRGB
//! numbers, never converted: the world is drawn the way Lantern Studio
//! composites, so a sprite looks in the game as it did when it was drawn.

use lntrn_image::Image;

/// How wide every sheet is.
pub const WIDTH: u32 = 2048;
/// Sprites sit on a grid this coarse, with at least this many clear
/// pixels on every side, so the smaller copies of two sprites never mix.
pub const GUTTER: u32 = 16;
/// The sheet and each halving of it the GPU is given.
pub const LEVELS: u32 = 5;

/// Part of a picture to pack: `[x, y, width, height]` in its pixels.
pub struct Source<'a> {
    pub image: &'a Image,
    pub rect: [u32; 4],
}

/// The packed sheet: level 0 first, each level half the one before.
pub struct Sheet {
    pub width: u32,
    pub height: u32,
    pub levels: Vec<Vec<u8>>,
}

impl Sheet {
    /// A sheet with nothing on it, for when no sprite could be loaded.
    pub fn empty() -> Sheet {
        build(&[]).expect("nothing always fits").0
    }
}

fn up_to_gutter(v: u32) -> u32 {
    v.div_ceil(GUTTER) * GUTTER
}

/// Where each of `sizes` (width, height) goes on a sheet `width` wide,
/// and how tall the sheet must be (a power of two). Tallest first, in
/// rows. `None` when one is too wide to fit at all.
pub fn pack(sizes: &[(u32, u32)], width: u32) -> Option<(Vec<(u32, u32)>, u32)> {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(sizes[i].1));
    let mut places = vec![(0, 0); sizes.len()];
    // The cell a sprite takes: itself and a gutter on each side.
    let (mut x, mut y, mut row_h) = (0, 0, 0);
    for i in order {
        let (w, h) = (up_to_gutter(sizes[i].0) + 2 * GUTTER, up_to_gutter(sizes[i].1) + 2 * GUTTER);
        if w > width {
            return None;
        }
        if x + w > width {
            (x, y, row_h) = (0, y + row_h, 0);
        }
        places[i] = (x + GUTTER, y + GUTTER);
        x += w;
        row_h = row_h.max(h);
    }
    let smallest = GUTTER << (LEVELS - 1);
    Some((places, (y + row_h).max(smallest).next_power_of_two()))
}

/// Pack `sources` into a sheet. Returns it and where each one landed
/// (its top-left corner), or `None` when one is too wide.
pub fn build(sources: &[Source]) -> Option<(Sheet, Vec<(u32, u32)>)> {
    let sizes: Vec<(u32, u32)> = sources.iter().map(|s| (s.rect[2], s.rect[3])).collect();
    let (places, height) = pack(&sizes, WIDTH)?;
    let mut base = vec![0u8; (WIDTH * height * 4) as usize];
    for (source, &(px, py)) in sources.iter().zip(&places) {
        let [sx, sy, w, h] = source.rect;
        for row in 0..h {
            for col in 0..w {
                let [r, g, b, a] = source.image.pixel(sx + col, sy + row);
                let at = (((py + row) * WIDTH + px + col) * 4) as usize;
                base[at..at + 4].copy_from_slice(&[premultiply(r, a), premultiply(g, a), premultiply(b, a), a]);
            }
        }
    }
    let mut levels = vec![base];
    let (mut w, mut h) = (WIDTH, height);
    for _ in 1..LEVELS {
        levels.push(half(levels.last().expect("level 0 is there"), w, h));
        (w, h) = (w / 2, h / 2);
    }
    Some((Sheet { width: WIDTH, height, levels }, places))
}

fn premultiply(c: u8, a: u8) -> u8 {
    ((c as u32 * a as u32 + 127) / 255) as u8
}

/// A picture of even sides at half the size: each pixel the mean of the
/// four it covers. Right for premultiplied colour, which this is.
fn half(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    let (w, h) = ((width / 2) as usize, (height / 2) as usize);
    let stride = width as usize * 4;
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let at = y * 2 * stride + x * 2 * 4;
            for c in 0..4 {
                let sum = pixels[at + c] as u32 + pixels[at + 4 + c] as u32 + pixels[at + stride + c] as u32 + pixels[at + stride + 4 + c] as u32;
                out[(y * w + x) * 4 + c] = ((sum + 2) / 4) as u8;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZES: [(u32, u32); 7] = [(392, 426), (236, 216), (164, 168), (164, 168), (48, 40), (1, 1), (1900, 30)];

    #[test]
    fn nothing_overlaps_and_every_gutter_is_kept() {
        let (places, height) = pack(&SIZES, WIDTH).expect("they fit");
        assert!(height.is_power_of_two());
        for (i, (&(w, h), &(x, y))) in SIZES.iter().zip(&places).enumerate() {
            assert_eq!((x % GUTTER, y % GUTTER), (0, 0), "sprite {i} is off the grid");
            assert!(x >= GUTTER && y >= GUTTER && x + w + GUTTER <= WIDTH && y + h + GUTTER <= height, "sprite {i} crowds an edge");
            for (j, (&(w2, h2), &(x2, y2))) in SIZES.iter().zip(&places).enumerate().skip(i + 1) {
                let apart_x = x + w + 2 * GUTTER <= x2 || x2 + w2 + 2 * GUTTER <= x;
                let apart_y = y + h + 2 * GUTTER <= y2 || y2 + h2 + 2 * GUTTER <= y;
                assert!(apart_x || apart_y, "sprites {i} and {j} are closer than two gutters");
            }
        }
    }

    #[test]
    fn too_wide_does_not_fit() {
        assert!(pack(&[(WIDTH, 10)], WIDTH).is_none());
        assert!(pack(&[(WIDTH - 2 * GUTTER, 10)], WIDTH).is_some());
    }

    #[test]
    fn an_empty_sheet_still_has_every_level() {
        let sheet = Sheet::empty();
        assert_eq!(sheet.levels.len(), LEVELS as usize);
        let (mut w, mut h) = (sheet.width, sheet.height);
        for level in &sheet.levels {
            assert_eq!(level.len(), (w * h * 4) as usize);
            (w, h) = (w / 2, h / 2);
        }
        assert!(h >= 1, "the smallest level has pixels");
    }

    #[test]
    fn pixels_land_premultiplied_where_they_were_placed() {
        let image = Image::new(2, 1, vec![200, 100, 50, 255, 200, 100, 50, 128]);
        // Only the right-hand pixel is asked for.
        let (sheet, places) = build(&[Source { image: &image, rect: [1, 0, 1, 1] }]).expect("fits");
        let (x, y) = places[0];
        let at = ((y * sheet.width + x) * 4) as usize;
        assert_eq!(sheet.levels[0][at..at + 4], [100, 50, 25, 128]);
        // And nothing else was written.
        assert_eq!(sheet.levels[0].iter().filter(|&&b| b != 0).count(), 4);
    }

    #[test]
    fn halving_averages_each_block_of_four() {
        let pixels = [[0u8, 0, 0, 0], [255, 255, 255, 255], [10, 20, 30, 40], [11, 21, 31, 41]].concat();
        assert_eq!(half(&pixels, 2, 2), vec![69, 74, 79, 84]);
    }

    /// The measure the gutter exists for: in every smaller copy, the
    /// texels a sprite can be sampled from (its own and one beyond, for
    /// the filter) hold nothing of any other sprite.
    #[test]
    fn no_smaller_copy_mixes_two_sprites() {
        let (places, _) = pack(&SIZES, WIDTH).expect("they fit");
        for level in 0..LEVELS {
            let texel = 1 << level;
            // The texels a sprite's pixels fall in at this level.
            let covers = |i: usize| {
                let ((w, h), (x, y)) = (SIZES[i], places[i]);
                (x / texel, y / texel, (x + w - 1) / texel, (y + h - 1) / texel)
            };
            for i in 0..SIZES.len() {
                let (ax0, ay0, ax1, ay1) = covers(i);
                for j in 0..SIZES.len() {
                    if i == j {
                        continue;
                    }
                    let (bx0, by0, bx1, by1) = covers(j);
                    let apart = ax1 + 1 < bx0 || bx1 + 1 < ax0 || ay1 + 1 < by0 || by1 + 1 < ay0;
                    assert!(apart, "level {level}: sprite {j} reaches into what sprite {i} is sampled from");
                }
            }
        }
    }
}
