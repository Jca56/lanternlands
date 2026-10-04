//! Maps on disk: a folder with one small binary file per chunk, named
//! for the chunk (`-1_0.chunk`).
//!
//! A chunk file, little-endian throughout:
//!
//! ```text
//! "CHNK"  version u16  side u8  sections u8
//! then each section:  tag [u8; 4]  length u32  payload
//!
//! "TERR"  names: count u8, then each as length u8 + UTF-8
//!         runs:  length u16, name index u8 (255 = no ground),
//!                until every tile is covered, row by row
//! "PROP"  names likewise
//!         count u16, then each prop: x u8, y u8, name index u8
//! ```
//!
//! Ground and props are written by name, not number, so kinds can be
//! added or reordered without breaking a saved map. A section this
//! build does not know is skipped; a name it does not know is an error,
//! never a silent hole in the map.

use std::fs;
use std::path::Path;

use crate::terrain::Terrain;
use crate::world::{AREA, CHUNK, Chunk, Prop, PropKind, VOID, World};

pub const EXTENSION: &str = "chunk";
const MAGIC: [u8; 4] = *b"CHNK";
const VERSION: u16 = 1;
const TERRAIN: [u8; 4] = *b"TERR";
const PROPS: [u8; 4] = *b"PROP";

fn file_name(key: (i32, i32)) -> String {
    format!("{}_{}.{EXTENSION}", key.0, key.1)
}

/// The chunk a file is named for.
fn key_of(file_name: &str) -> Option<(i32, i32)> {
    let (x, y) = file_name.strip_suffix(EXTENSION)?.strip_suffix('.')?.split_once('_')?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

/// The names in use and, for each item, its place among them. `None`
/// items (no ground) get [`VOID`].
fn name_table(items: impl Iterator<Item = Option<&'static str>>) -> (Vec<&'static str>, Vec<u8>) {
    let mut names: Vec<&'static str> = Vec::new();
    let indices = items
        .map(|item| match item {
            None => VOID,
            Some(name) => names.iter().position(|n| *n == name).unwrap_or_else(|| {
                names.push(name);
                names.len() - 1
            }) as u8,
        })
        .collect();
    (names, indices)
}

fn write_names(out: &mut Vec<u8>, names: &[&str]) {
    out.push(names.len() as u8);
    for name in names {
        out.push(name.len() as u8);
        out.extend_from_slice(name.as_bytes());
    }
}

pub(crate) fn encode(chunk: &Chunk) -> Vec<u8> {
    let mut terrain = Vec::new();
    let (names, indices) = name_table(chunk.tiles.iter().map(|&rank| Terrain::from_rank(rank).map(Terrain::name)));
    write_names(&mut terrain, &names);
    for run in indices.chunk_by(|a, b| a == b) {
        terrain.extend_from_slice(&(run.len() as u16).to_le_bytes());
        terrain.push(run[0]);
    }

    let mut props = Vec::new();
    // In a fixed order, so the same chunk is always the same bytes.
    let mut standing = chunk.props.clone();
    standing.sort_by_key(|p| (p.y, p.x));
    let (names, indices) = name_table(standing.iter().map(|p| Some(p.kind.name())));
    write_names(&mut props, &names);
    props.extend_from_slice(&(standing.len() as u16).to_le_bytes());
    for (prop, index) in standing.iter().zip(indices) {
        props.extend_from_slice(&[prop.x.rem_euclid(CHUNK) as u8, prop.y.rem_euclid(CHUNK) as u8, index]);
    }

    let mut sections = vec![(TERRAIN, terrain)];
    if !standing.is_empty() {
        sections.push((PROPS, props));
    }
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&[CHUNK as u8, sections.len() as u8]);
    for (tag, payload) in sections {
        out.extend_from_slice(&tag);
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(&payload);
    }
    out
}

/// Bytes read front to back; running out is an error, not a panic.
struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if n > self.bytes.len() {
            return Err("the file ends early".to_owned());
        }
        let (head, rest) = self.bytes.split_at(n);
        self.bytes = rest;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("two bytes")))
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("four bytes")))
    }

    fn tag(&mut self) -> Result<[u8; 4], String> {
        Ok(self.take(4)?.try_into().expect("four bytes"))
    }

    /// A name table, each name turned into what it names by `find`.
    fn names<T>(&mut self, what: &str, find: impl Fn(&str) -> Option<T>) -> Result<Vec<T>, String> {
        (0..self.u8()?)
            .map(|_| {
                let length = self.u8()? as usize;
                let name = std::str::from_utf8(self.take(length)?).map_err(|_| format!("a {what} name is not text"))?;
                find(name).ok_or(format!("this build has no {what} called {name:?}"))
            })
            .collect()
    }

    fn done(&self, what: &str) -> Result<(), String> {
        if self.bytes.is_empty() { Ok(()) } else { Err(format!("{} bytes left over after {what}", self.bytes.len())) }
    }
}

/// The chunk at `key` from its file's bytes.
pub(crate) fn decode(bytes: &[u8], key: (i32, i32)) -> Result<Chunk, String> {
    let mut file = Reader { bytes };
    if file.tag()? != MAGIC {
        return Err("not a chunk file".to_owned());
    }
    let version = file.u16()?;
    if version != VERSION {
        return Err(format!("it is version {version}; this build reads version {VERSION}"));
    }
    let side = file.u8()?;
    if side as i32 != CHUNK {
        return Err(format!("its chunks are {side} tiles a side, not {CHUNK}"));
    }
    let mut chunk = Chunk::new();
    let mut seen: Vec<[u8; 4]> = Vec::new();
    for _ in 0..file.u8()? {
        let tag = file.tag()?;
        let length = file.u32()? as usize;
        let mut section = Reader { bytes: file.take(length)? };
        if seen.contains(&tag) {
            return Err(format!("it has two {:?} sections", String::from_utf8_lossy(&tag)));
        }
        seen.push(tag);
        match tag {
            TERRAIN => read_terrain(&mut section, &mut chunk)?,
            // Props need the ground they stand on, which is written first.
            PROPS if seen.contains(&TERRAIN) => read_props(&mut section, &mut chunk, key)?,
            PROPS => return Err("its props come before its ground".to_owned()),
            _ => {}
        }
    }
    file.done("the last section")?;
    if !seen.contains(&TERRAIN) {
        return Err("it has no ground section".to_owned());
    }
    Ok(chunk)
}

fn read_terrain(section: &mut Reader, chunk: &mut Chunk) -> Result<(), String> {
    let names = section.names("ground", Terrain::from_name)?;
    let mut at = 0;
    while at < AREA {
        let (length, index) = (section.u16()? as usize, section.u8()?);
        let rank = match index {
            VOID => VOID,
            i => names.get(i as usize).ok_or(format!("a run uses ground number {i} of {}", names.len()))?.rank(),
        };
        if length == 0 || at + length > AREA {
            return Err(format!("a run of {length} tiles does not fit at tile {at}"));
        }
        chunk.tiles[at..at + length].fill(rank);
        at += length;
    }
    section.done("the ground")
}

fn read_props(section: &mut Reader, chunk: &mut Chunk, key: (i32, i32)) -> Result<(), String> {
    let names = section.names("prop", PropKind::from_name)?;
    for _ in 0..section.u16()? {
        let (x, y, index) = (section.u8()? as i32, section.u8()? as i32, section.u8()?);
        let kind = *names.get(index as usize).ok_or(format!("a prop is kind number {index} of {}", names.len()))?;
        if x >= CHUNK || y >= CHUNK {
            return Err(format!("a prop stands at ({x}, {y}), outside the chunk"));
        }
        if chunk.tiles[(y * CHUNK + x) as usize] == VOID {
            return Err(format!("a prop stands at ({x}, {y}), where there is no ground"));
        }
        let (x, y) = (key.0 * CHUNK + x, key.1 * CHUNK + y);
        if chunk.props.iter().any(|p| (p.x, p.y) == (x, y)) {
            return Err(format!("two props stand on tile ({x}, {y})"));
        }
        chunk.props.push(Prop { kind, x, y });
    }
    section.done("the props")
}

impl World {
    /// Write every chunk changed since the last load or save into `dir`,
    /// and take away the files of chunks that have nothing left in them.
    /// Returns how many files were written or removed. Each file is
    /// written whole beside its place and then moved into it, so a crash
    /// never leaves half a chunk.
    pub fn save(&mut self, dir: &Path) -> Result<usize, String> {
        let io = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());
        fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
        let mut touched = 0;
        while let Some(&key) = self.dirty.first() {
            let path = dir.join(file_name(key));
            match self.chunks.get(&key).filter(|chunk| !chunk.is_empty()) {
                Some(chunk) => {
                    let scratch = dir.join(format!(".{}.tmp", file_name(key)));
                    fs::write(&scratch, encode(chunk)).map_err(|e| io(&scratch, e))?;
                    fs::rename(&scratch, &path).map_err(|e| io(&path, e))?;
                    touched += 1;
                }
                None => {
                    self.chunks.remove(&key);
                    match fs::remove_file(&path) {
                        Ok(()) => touched += 1,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(io(&path, e)),
                    }
                }
            }
            self.dirty.remove(&key);
        }
        Ok(touched)
    }

    /// The map in `dir`: every chunk file there. A folder that does not
    /// exist is a map nobody has drawn yet, an empty world. Anything
    /// wrong with a file is an error naming it, and nothing is loaded.
    pub fn load(dir: &Path) -> Result<World, String> {
        let mut world = World::new();
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(world),
            Err(e) => return Err(format!("{}: {e}", dir.display())),
        };
        for entry in entries {
            let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            if path.extension().is_none_or(|ext| ext != EXTENSION) {
                continue;
            }
            let wrong = |why: String| format!("{}: {why}", path.display());
            let key = path.file_name().and_then(|name| key_of(name.to_str()?)).ok_or_else(|| wrong("its name is not a chunk's, like 3_-1.chunk".to_owned()))?;
            let bytes = fs::read(&path).map_err(|e| wrong(e.to_string()))?;
            world.chunks.insert(key, decode(&bytes, key).map_err(wrong)?);
        }
        Ok(world)
    }

    /// How many chunks hold anything.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Chunk {
        let mut chunk = Chunk::new();
        for (i, tile) in chunk.tiles.iter_mut().enumerate() {
            let (x, y) = (i as i32 % CHUNK, i as i32 / CHUNK);
            *tile = match (x, y) {
                (0..4, _) => VOID,
                (_, 10..14) => Terrain::Water.rank(),
                (20.., 20..) => Terrain::Sand.rank(),
                _ => Terrain::Grass.rank(),
            };
        }
        // Chunk (-2, 3): its tiles start at (-64, 96).
        chunk.props = vec![Prop { kind: PropKind::Rock, x: -64 + 30, y: 96 + 2 }, Prop { kind: PropKind::Tree, x: -64 + 5, y: 96 }, Prop { kind: PropKind::HerbCyan, x: -64 + 31, y: 96 + 31 }];
        chunk
    }

    fn same(a: &Chunk, b: &Chunk) -> bool {
        let sorted = |c: &Chunk| {
            let mut props = c.props.clone();
            props.sort_by_key(|p| (p.y, p.x));
            props
        };
        a.tiles == b.tiles && sorted(a) == sorted(b)
    }

    #[test]
    fn a_chunk_comes_back_as_it_went() {
        let chunk = sample();
        let back = decode(&encode(&chunk), (-2, 3)).expect("decodes");
        assert!(same(&chunk, &back));
        // An empty chunk and a full one too.
        for chunk in [Chunk::new(), Chunk { tiles: Box::new([Terrain::Dirt.rank(); AREA]), props: Vec::new() }] {
            assert!(same(&chunk, &decode(&encode(&chunk), (0, 0)).expect("decodes")));
        }
    }

    #[test]
    fn the_same_chunk_is_always_the_same_bytes() {
        let (a, mut b) = (sample(), sample());
        b.props.reverse();
        assert_eq!(encode(&a), encode(&b));
    }

    #[test]
    fn plain_ground_takes_a_handful_of_bytes() {
        let grass = Chunk { tiles: Box::new([Terrain::Grass.rank(); AREA]), props: Vec::new() };
        assert!(encode(&grass).len() < 40, "{} bytes for one run of grass", encode(&grass).len());
        assert!(encode(&sample()).len() < 400, "{} bytes", encode(&sample()).len());
    }

    #[test]
    fn names_not_numbers_are_what_is_stored() {
        let bytes = encode(&sample());
        let has = |name: &str| bytes.windows(name.len()).any(|w| w == name.as_bytes());
        assert!(has("grass") && has("water") && has("sand") && has("tree") && has("herb_cyan"));
        assert!(!has("dirt"), "a ground the chunk does not use is not named");
    }

    #[test]
    fn a_damaged_file_is_refused_with_a_reason() {
        let good = encode(&sample());
        let refuse = |bytes: &[u8]| decode(bytes, (0, 0)).err().expect("refused");
        assert!(refuse(b"PNG nonsense").contains("not a chunk file"));
        assert!(refuse(&good[..good.len() - 1]).contains("ends early"));
        assert!(refuse(&[&good[..], &[0]].concat()).contains("left over"));
        let mut newer = good.clone();
        newer[4] = 9;
        assert!(refuse(&newer).contains("version 9"));
        // Every way of cutting it short is an error, never a panic.
        for cut in 0..good.len() {
            assert!(decode(&good[..cut], (0, 0)).is_err(), "cut at {cut}");
        }
    }

    #[test]
    fn an_unknown_name_is_an_error_not_a_hole() {
        let mut bytes = encode(&sample());
        let at = bytes.windows(5).position(|w| w == b"water").expect("water is named");
        bytes[at..at + 5].copy_from_slice(b"magma");
        let why = decode(&bytes, (0, 0)).err().expect("refused");
        assert!(why.contains("no ground called \"magma\""), "{why}");
    }

    #[test]
    fn a_section_from_the_future_is_skipped() {
        let mut bytes = encode(&sample());
        bytes[7] += 1;
        bytes.extend_from_slice(b"WTHR");
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3]);
        assert!(same(&sample(), &decode(&bytes, (-2, 3)).expect("decodes")));
    }

    #[test]
    fn file_names_carry_the_chunk_and_nothing_else_parses() {
        for key in [(0, 0), (-1, 0), (12, -7), (-100, -100)] {
            assert_eq!(key_of(&file_name(key)), Some(key));
        }
        for name in ["0_0.txt", "0.chunk", "a_b.chunk", "1_2_3.chunk", "1_2chunk", ".1_2.chunk.tmp"] {
            assert_eq!(key_of(name), None, "{name}");
        }
    }
}
