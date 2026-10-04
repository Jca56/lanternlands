//! Finding and loading what is in `assets/`: the sprite list
//! (`sprites.toml`) and the pictures it names, packed into one sheet.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lntrn_data::Doc;
use lntrn_image::Image;
use lntrn_math::Vec2;

use crate::atlas::{self, Sheet, Source};

pub const MANIFEST: &str = "sprites.toml";

/// Where the assets are: `LANTERNLANDS_ASSETS` if it is set, else an `assets`
/// folder beside the binary, else the one in the source tree.
pub fn assets_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("LANTERNLANDS_ASSETS") {
        return dir.into();
    }
    let beside = std::env::current_exe().ok().and_then(|exe| Some(exe.parent()?.join("assets")));
    match beside {
        Some(dir) if dir.is_dir() => dir,
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"),
    }
}

/// One `[sprites.name]` table.
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteDef {
    pub name: String,
    /// The picture, as a path inside the assets folder.
    pub file: String,
    /// `[x, y, width, height]` in the picture; all of it when `None`.
    pub rect: Option<[u32; 4]>,
    /// The pixel, counted from the rect's corner, that stands on the ground.
    pub anchor: [f64; 2],
    /// A grid's cell: packed as only the part of it that is drawn, and
    /// no sprite at all when nothing is.
    pub trim: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Manifest {
    /// How many of the art's pixels a tile is across.
    pub tile_px: f64,
    pub sprites: Vec<SpriteDef>,
}

fn numbers<const N: usize>(doc: &Doc) -> Option<[f64; N]> {
    let list = doc.as_list()?;
    let mut out = [0.0; N];
    (list.len() == N).then_some(())?;
    for (slot, item) in out.iter_mut().zip(list) {
        *slot = item.as_f64()?;
    }
    Some(out)
}

fn strings(doc: &Doc) -> Option<Vec<&str>> {
    doc.as_list()?.iter().map(Doc::as_str).collect()
}

/// A `[grids.name]` table as the sprites it stands for: one per cell.
fn grid_sprites(name: &str, entry: &Doc) -> Result<Vec<SpriteDef>, String> {
    let whole = |v: &[f64; 2]| v.iter().all(|v| *v >= 1.0 && v.fract() == 0.0);
    let file = entry.get("file").and_then(Doc::as_str).ok_or(format!("{name}: no file"))?;
    let [w, h] = entry.get("cell").and_then(numbers::<2>).filter(whole).ok_or(format!("{name}: cell must be [width, height] in whole pixels"))?.map(|v| v as u32);
    let anchor = entry.get("anchor").and_then(numbers::<2>).ok_or(format!("{name}: anchor must be [x, y]"))?;
    let rows = entry.get("rows").and_then(strings).ok_or(format!("{name}: rows must be a list of names"))?;
    let columns = entry.get("columns").and_then(strings).ok_or(format!("{name}: columns must be a list of names"))?;
    let cell = |r: usize, c: usize| SpriteDef { name: format!("{name}_{}_{}", rows[r], columns[c]), file: file.to_owned(), rect: Some([c as u32 * w, r as u32 * h, w, h]), anchor, trim: true };
    Ok((0..rows.len()).flat_map(|r| (0..columns.len()).map(move |c| (r, c))).map(|(r, c)| cell(r, c)).collect())
}

pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let doc = lntrn_data::toml::parse(text).map_err(|e| e.to_string())?;
    let tile_px = doc.get("tile_px").and_then(Doc::as_f64).filter(|px| *px > 0.0).ok_or("tile_px is missing or is not a size")?;
    let table = doc.get("sprites").and_then(Doc::as_map).ok_or("there is no [sprites] table")?;
    let mut sprites = Vec::new();
    for (name, entry) in table.iter() {
        let file = entry.get("file").and_then(Doc::as_str).ok_or(format!("{name}: no file"))?.to_owned();
        let anchor = entry.get("anchor").and_then(numbers::<2>).ok_or(format!("{name}: anchor must be [x, y]"))?;
        let rect = match entry.get("rect") {
            None => None,
            Some(doc) => {
                let r = numbers::<4>(doc).filter(|r| r.iter().all(|v| *v >= 0.0 && v.fract() == 0.0)).ok_or(format!("{name}: rect must be [x, y, width, height] in whole pixels"))?;
                Some(r.map(|v| v as u32))
            }
        };
        sprites.push(SpriteDef { name: name.to_owned(), file, rect, anchor, trim: false });
    }
    if let Some(grids) = doc.get("grids").and_then(Doc::as_map) {
        for (name, entry) in grids.iter() {
            sprites.extend(grid_sprites(name, entry)?);
        }
    }
    Ok(Manifest { tile_px, sprites })
}

/// A sprite, ready to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    /// Where it is on the sheet: `[u0, v0, u1, v1]`.
    pub uv: [f32; 4],
    /// Its size in tiles.
    pub size: Vec2,
    /// From its top-left corner to the point that stands on the ground,
    /// in tiles.
    pub anchor: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpriteId(u32);

/// Every sprite, on one sheet.
pub struct SpriteSet {
    pub sheet: Sheet,
    sprites: Vec<Sprite>,
    /// Each sprite cut out on its own, as it was drawn: for the interface.
    pictures: Vec<Image>,
    names: HashMap<String, SpriteId>,
}

impl SpriteSet {
    /// No sprites at all: what the game runs with when loading failed.
    pub fn empty() -> SpriteSet {
        SpriteSet { sheet: Sheet::empty(), sprites: Vec::new(), pictures: Vec::new(), names: HashMap::new() }
    }

    pub fn id(&self, name: &str) -> Option<SpriteId> {
        self.names.get(name).copied()
    }

    pub fn get(&self, id: SpriteId) -> &Sprite {
        &self.sprites[id.0 as usize]
    }

    /// The sprite as a picture of its own.
    pub fn picture(&self, id: SpriteId) -> &Image {
        &self.pictures[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.sprites.len()
    }

    /// Read the manifest in `dir` and every picture it names.
    pub fn load(dir: &Path) -> Result<SpriteSet, String> {
        let path = dir.join(MANIFEST);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let manifest = parse_manifest(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        SpriteSet::from_manifest(&manifest, |file| {
            let path = dir.join(file);
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            lntrn_image::decode(&bytes).map_err(|e| format!("{}: {e}", path.display()))
        })
    }

    /// Pack the manifest's sprites, getting each picture from `open`
    /// (once, however many sprites are cut from it).
    pub fn from_manifest(manifest: &Manifest, mut open: impl FnMut(&str) -> Result<Image, String>) -> Result<SpriteSet, String> {
        let mut files: Vec<(&str, Image)> = Vec::new();
        for def in &manifest.sprites {
            if !files.iter().any(|(file, _)| *file == def.file) {
                files.push((&def.file, open(&def.file)?));
            }
        }
        let mut sources = Vec::new();
        // The sprites that are packed, each with its anchor in the rect
        // it is packed as.
        let mut kept: Vec<(&SpriteDef, [f64; 2])> = Vec::new();
        for def in &manifest.sprites {
            let image = &files.iter().find(|(file, _)| *file == def.file).expect("opened above").1;
            let rect = def.rect.unwrap_or([0, 0, image.width, image.height]);
            let [x, y, w, h] = rect;
            let inside = w > 0 && h > 0 && x.checked_add(w).is_some_and(|r| r <= image.width) && y.checked_add(h).is_some_and(|b| b <= image.height);
            if !inside {
                return Err(format!("{}: rect {rect:?} is not inside {} ({}x{})", def.name, def.file, image.width, image.height));
            }
            let (rect, anchor) = if def.trim {
                // A cell with nothing drawn in it is no sprite.
                let Some(drawn) = drawn_part(image, rect) else { continue };
                (drawn, [def.anchor[0] - (drawn[0] - x) as f64, def.anchor[1] - (drawn[1] - y) as f64])
            } else {
                (rect, def.anchor)
            };
            kept.push((def, anchor));
            sources.push(Source { image, rect });
        }
        let (sheet, places) = atlas::build(&sources).ok_or("a sprite is too wide for the sheet")?;
        let pictures = sources.iter().map(|s| s.image.crop(s.rect[0], s.rect[1], s.rect[2], s.rect[3])).collect();

        let (sheet_w, sheet_h) = (sheet.width as f64, sheet.height as f64);
        let mut set = SpriteSet { sheet, sprites: Vec::new(), pictures, names: HashMap::new() };
        for ((&(def, anchor), source), &(x, y)) in kept.iter().zip(&sources).zip(&places) {
            let (x, y, w, h) = (x as f64, y as f64, source.rect[2] as f64, source.rect[3] as f64);
            let id = SpriteId(set.sprites.len() as u32);
            if set.names.insert(def.name.clone(), id).is_some() {
                return Err(format!("{}: named twice", def.name));
            }
            set.sprites.push(Sprite {
                uv: [(x / sheet_w) as f32, (y / sheet_h) as f32, ((x + w) / sheet_w) as f32, ((y + h) / sheet_h) as f32],
                size: Vec2::new(w, h) / manifest.tile_px,
                anchor: Vec2::new(anchor[0], anchor[1]) / manifest.tile_px,
            });
        }
        Ok(set)
    }
}

/// The smallest box inside `rect` holding every pixel of it that is not
/// fully clear; `None` when they all are.
fn drawn_part(image: &Image, [x, y, w, h]: [u32; 4]) -> Option<[u32; 4]> {
    let drawn = |px: u32, py: u32| image.rgba[(py as usize * image.width as usize + px as usize) * 4 + 3] != 0;
    let rows = || (y..y + h).filter(|&py| (x..x + w).any(|px| drawn(px, py)));
    let (top, bottom) = (rows().next()?, rows().next_back()?);
    let columns = || (x..x + w).filter(|&px| (top..=bottom).any(|py| drawn(px, py)));
    let (left, right) = (columns().next()?, columns().next_back()?);
    Some([left, top, right - left + 1, bottom - top + 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = r#"
tile_px = 200

[sprites.tree]
file = "sprites/objects.png"
rect = [4, 24, 40, 60]
anchor = [20, 50]

[sprites.tuft]
file = "sprites/tuft.png"
anchor = [4.5, 6]
"#;

    fn picture(file: &str) -> Result<Image, String> {
        match file {
            "sprites/objects.png" => Ok(Image::solid(100, 100, [10, 20, 30, 255])),
            "sprites/tuft.png" => Ok(Image::solid(8, 8, [40, 50, 60, 255])),
            other => Err(format!("{other}: no such file")),
        }
    }

    #[test]
    fn the_manifest_reads_rects_anchors_and_the_tile_size() {
        let manifest = parse_manifest(TEXT).expect("parses");
        assert_eq!(manifest.tile_px, 200.0);
        assert_eq!(manifest.sprites.len(), 2);
        assert_eq!(manifest.sprites[0], SpriteDef { name: "tree".into(), file: "sprites/objects.png".into(), rect: Some([4, 24, 40, 60]), anchor: [20.0, 50.0], trim: false });
        assert_eq!(manifest.sprites[1].rect, None);
        assert_eq!(manifest.sprites[1].anchor, [4.5, 6.0]);
    }

    #[test]
    fn a_grid_is_one_sprite_per_cell_named_for_its_row_and_column() {
        let text = format!("{TEXT}\n[grids.hero]\nfile = \"sprites/objects.png\"\ncell = [20, 30]\nanchor = [10, 28]\nrows = [\"down\", \"up\"]\ncolumns = [\"idle\", \"step\", \"pass\"]\n");
        let manifest = parse_manifest(&text).expect("parses");
        let grid: Vec<&SpriteDef> = manifest.sprites.iter().filter(|s| s.name.starts_with("hero_")).collect();
        assert_eq!(grid.len(), 6);
        assert_eq!(*grid[0], SpriteDef { name: "hero_down_idle".into(), file: "sprites/objects.png".into(), rect: Some([0, 0, 20, 30]), anchor: [10.0, 28.0], trim: true });
        assert_eq!((grid[5].name.as_str(), grid[5].rect), ("hero_up_pass", Some([40, 30, 20, 30])));
        // And they load like any other sprite.
        let set = SpriteSet::from_manifest(&manifest, picture).expect("loads");
        assert_eq!(set.get(set.id("hero_up_step").unwrap()).size, Vec2::new(0.1, 0.15));
        let broken = text.replace("cell = [20, 30]", "cell = [20.5, 30]");
        assert!(parse_manifest(&broken).unwrap_err().contains("hero: cell"));
    }

    #[test]
    fn a_grids_cell_is_packed_as_only_what_is_drawn_in_it_and_an_empty_cell_is_no_sprite() {
        // A 40 by 30 picture cut into two cells: the first has a 6 by 4
        // box drawn at (5, 10), the second nothing at all.
        let mut image = Image::solid(40, 30, [0, 0, 0, 0]);
        for py in 10..14 {
            for px in 5..11 {
                let at = (py * 40 + px) * 4;
                image.rgba[at..at + 4].copy_from_slice(&[200, 100, 50, 255]);
            }
        }
        let text = "tile_px = 1\n[sprites]\n[grids.hat]\nfile = \"hat.png\"\ncell = [20, 30]\nanchor = [10, 28]\nrows = [\"down\"]\ncolumns = [\"idle\", \"step\"]\n";
        let set = SpriteSet::from_manifest(&parse_manifest(text).expect("parses"), |_| Ok(image.clone())).expect("loads");
        assert_eq!(set.len(), 1);
        assert_eq!(set.id("hat_down_step"), None, "nothing drawn, no sprite");
        let hat = set.get(set.id("hat_down_idle").expect("the drawn cell"));
        assert_eq!(hat.size, Vec2::new(6.0, 4.0));
        // Its anchor is still the same point of the picture: (10, 28)
        // of the cell is (5, 18) from the box's corner.
        assert_eq!(hat.anchor, Vec2::new(5.0, 18.0));
        let picture = set.picture(set.id("hat_down_idle").unwrap());
        assert_eq!((picture.width, picture.height, picture.pixel(0, 0), picture.pixel(5, 3)), (6, 4, [200, 100, 50, 255], [200, 100, 50, 255]));
    }

    #[test]
    fn a_broken_manifest_says_what_is_wrong() {
        assert!(parse_manifest("[sprites.a]\nfile = \"a.png\"\nanchor = [0, 0]").unwrap_err().contains("tile_px"));
        assert!(parse_manifest("tile_px = 200").unwrap_err().contains("[sprites]"));
        assert!(parse_manifest("tile_px = 200\n[sprites.a]\nanchor = [0, 0]").unwrap_err().contains("a: no file"));
        assert!(parse_manifest("tile_px = 200\n[sprites.a]\nfile = \"a.png\"\nanchor = [1]").unwrap_err().contains("anchor"));
        assert!(parse_manifest("tile_px = 200\n[sprites.a]\nfile = \"a.png\"\nanchor = [1, 2]\nrect = [0, 0, 1.5, 2]").unwrap_err().contains("rect"));
    }

    #[test]
    fn sprites_come_out_in_tiles_with_their_place_on_the_sheet() {
        let set = SpriteSet::from_manifest(&parse_manifest(TEXT).unwrap(), picture).expect("loads");
        assert_eq!(set.len(), 2);
        let tree = set.get(set.id("tree").expect("there"));
        assert_eq!(tree.size, Vec2::new(0.2, 0.3));
        assert_eq!(tree.anchor, Vec2::new(0.1, 0.25));
        // The uv box is exactly the sprite's pixels on the sheet.
        let (w, h) = (set.sheet.width as f32, set.sheet.height as f32);
        assert_eq!(((tree.uv[2] - tree.uv[0]) * w).round(), 40.0);
        assert_eq!(((tree.uv[3] - tree.uv[1]) * h).round(), 60.0);
        // And those pixels are the picture's.
        let (x, y) = ((tree.uv[0] * w).round() as u32, (tree.uv[1] * h).round() as u32);
        let at = ((y * set.sheet.width + x) * 4) as usize;
        assert_eq!(set.sheet.levels[0][at..at + 4], [10, 20, 30, 255]);
        // Its own picture is the same pixels, cut out.
        let picture = set.picture(set.id("tree").unwrap());
        assert_eq!((picture.width, picture.height, picture.pixel(0, 0)), (40, 60, [10, 20, 30, 255]));
        // A sprite with no rect is its whole picture.
        assert_eq!(set.get(set.id("tuft").unwrap()).size, Vec2::new(0.04, 0.04));
        assert_eq!(set.id("goblin"), None);
    }

    #[test]
    fn a_rect_outside_its_picture_is_refused() {
        let text = TEXT.replace("[4, 24, 40, 60]", "[80, 24, 40, 60]");
        let err = SpriteSet::from_manifest(&parse_manifest(&text).unwrap(), picture).err().expect("refused");
        assert!(err.contains("tree") && err.contains("not inside"), "{err}");
    }

    #[test]
    fn a_missing_picture_is_an_error_with_its_name() {
        let text = TEXT.replace("sprites/tuft.png", "sprites/gone.png");
        let err = SpriteSet::from_manifest(&parse_manifest(&text).unwrap(), picture).err().expect("refused");
        assert!(err.contains("gone.png"), "{err}");
    }

    /// The real manifest and its pictures, as they are in the repository.
    fn shipped() -> (Manifest, std::path::PathBuf) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let text = std::fs::read_to_string(dir.join(MANIFEST)).expect("the manifest is there");
        (parse_manifest(&text).expect("the manifest parses"), dir)
    }

    #[test]
    fn the_shipped_assets_load() {
        let (manifest, dir) = shipped();
        let set = SpriteSet::load(&dir).expect("assets load");
        // Everything listed one by one is there; of the grids' cells,
        // those with something drawn in them.
        for def in manifest.sprites.iter().filter(|def| !def.trim) {
            assert!(set.id(&def.name).is_some(), "{} did not load", def.name);
        }
        let cells = manifest.sprites.iter().filter(|def| def.trim).count();
        let loaded = manifest.sprites.iter().filter(|def| def.trim && set.id(&def.name).is_some()).count();
        assert!(loaded > cells / 2 && loaded < cells, "{loaded} of {cells} cells have something drawn in them");
        assert_eq!(set.len(), manifest.sprites.len() - (cells - loaded));
    }

    /// The rects were measured by hand off the sheet, so check them
    /// against its pixels: every sprite's box has a clear border (nothing
    /// is cut off), and everything drawn on a sheet is inside exactly
    /// one box (nothing is left out or shared).
    #[test]
    fn every_rect_holds_one_whole_sprite() {
        let (manifest, dir) = shipped();
        let files: std::collections::BTreeSet<&str> = manifest.sprites.iter().map(|s| s.file.as_str()).collect();
        for file in files {
            let image = lntrn_image::decode(&std::fs::read(dir.join(file)).expect("the picture is there")).expect("it decodes");
            let rects: Vec<(&str, [u32; 4])> = manifest.sprites.iter().filter(|s| s.file == file).map(|s| (s.name.as_str(), s.rect.unwrap_or([0, 0, image.width, image.height]))).collect();
            for &(name, [x, y, w, h]) in &rects {
                let border = (x..x + w).flat_map(|px| [(px, y), (px, y + h - 1)]).chain((y..y + h).flat_map(|py| [(x, py), (x + w - 1, py)]));
                for (px, py) in border {
                    assert_eq!(image.pixel(px, py)[3], 0, "{name}: something is drawn on its rect's edge at ({px}, {py}) of {file}");
                }
                let drawn = (y..y + h).flat_map(|py| (x..x + w).map(move |px| (px, py))).filter(|&(px, py)| image.pixel(px, py)[3] > 0).count();
                // A piece of gear's cell may hold little, or nothing.
                assert!(drawn as u32 > w * h / 8 || name.starts_with("gear_"), "{name}: its rect in {file} is nearly empty");
            }
            for py in 0..image.height {
                for px in 0..image.width {
                    if image.pixel(px, py)[3] == 0 {
                        continue;
                    }
                    let holders = rects.iter().filter(|(_, [x, y, w, h])| px >= *x && px < x + w && py >= *y && py < y + h).count();
                    assert_eq!(holders, 1, "the pixel at ({px}, {py}) of {file} is in {holders} sprites' rects");
                }
            }
        }
    }

    /// An anchor is where the sprite touches the ground: inside its box,
    /// in the lower half, and not far off its middle. What lies flat on
    /// the ground is anchored by its own middle instead.
    #[test]
    fn every_anchor_is_at_the_foot_of_its_sprite() {
        let (manifest, dir) = shipped();
        const FLAT: [&str; 1] = ["target_ring"];
        for def in &manifest.sprites {
            let [w, h] = match def.rect {
                Some([_, _, w, h]) => [w as f64, h as f64],
                None => {
                    let image = lntrn_image::decode(&std::fs::read(dir.join(&def.file)).unwrap()).unwrap();
                    [image.width as f64, image.height as f64]
                }
            };
            let [ax, ay] = def.anchor;
            if FLAT.contains(&def.name.as_str()) {
                assert_eq!([ax, ay], [w * 0.5, h * 0.5], "{}: anchored off its middle", def.name);
                continue;
            }
            assert!(ay > h * 0.5 && ay <= h, "{}: anchor y {ay} is not in the lower half of {h}", def.name);
            assert!((ax - w * 0.5).abs() <= w * 0.2, "{}: anchor x {ax} is far from the middle of {w}", def.name);
        }
    }
}
