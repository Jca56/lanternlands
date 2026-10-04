//! Characters on disk: a few slots, one file each, in the game's
//! folder under Lantern's `save/`.

use std::path::{Path, PathBuf};

use sim::Saved;

/// How many characters there can be.
pub const SLOTS: usize = 3;

pub struct Saves {
    dir: PathBuf,
}

impl Saves {
    pub fn new(dir: PathBuf) -> Saves {
        Saves { dir }
    }

    /// `~/.lantern/save/<app_id>` (or under `$LANTERN_HOME`).
    pub fn default_dir(app_id: &str) -> PathBuf {
        lntrn_sys::dirs::lantern().unwrap_or_else(|| PathBuf::from(".lantern")).join("save").join(app_id)
    }

    #[cfg(test)]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, slot: usize) -> PathBuf {
        self.dir.join(format!("character-{}.save", slot + 1))
    }

    /// The character in a slot: `None` when the slot is empty, an error
    /// when its file is there and cannot be read.
    pub fn read(&self, slot: usize) -> Option<Result<Saved, String>> {
        let path = self.path(slot);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
            Err(e) => return Some(Err(format!("{}: {e}", path.display()))),
        };
        Some(Saved::decode(&bytes).map_err(|why| format!("{}: {why}", path.display())))
    }

    /// Put a character in a slot. The file is written whole beside its
    /// place and then moved into it, so a crash never leaves half a save.
    pub fn write(&self, slot: usize, saved: &Saved) -> Result<(), String> {
        let io = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());
        std::fs::create_dir_all(&self.dir).map_err(|e| io(&self.dir, e))?;
        let (path, scratch) = (self.path(slot), self.dir.join(format!(".character-{}.tmp", slot + 1)));
        std::fs::write(&scratch, saved.encode()).map_err(|e| io(&scratch, e))?;
        std::fs::rename(&scratch, &path).map_err(|e| io(&path, e))
    }

    /// Empty a slot. One that was empty already is no error.
    pub fn delete(&self, slot: usize) -> Result<(), String> {
        let path = self.path(slot);
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {e}", path.display())),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use sim::{Character, Sim, World};

    use super::*;

    fn saves(name: &str) -> Saves {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/saves").join(name);
        let _ = std::fs::remove_dir_all(&dir);
        Saves::new(dir)
    }

    fn saved(name: &str) -> Saved {
        Sim::new(World::new()).snapshot(&Character { name: name.to_owned(), played: 60 })
    }

    #[test]
    fn slots_are_empty_until_written_and_each_is_its_own_file() {
        let saves = saves("slots");
        assert!((0..SLOTS).all(|slot| saves.read(slot).is_none()));
        saves.write(1, &saved("Middle")).expect("writes");
        assert!(saves.read(0).is_none() && saves.read(2).is_none());
        assert_eq!(saves.read(1).expect("there").expect("reads").character.name, "Middle");
        let files: Vec<_> = std::fs::read_dir(saves.dir()).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        assert_eq!(files, ["character-2.save"], "no scratch file is left behind");
    }

    #[test]
    fn writing_again_replaces_and_deleting_empties() {
        let saves = saves("replace");
        saves.write(0, &saved("First")).unwrap();
        saves.write(0, &saved("Second")).unwrap();
        assert_eq!(saves.read(0).unwrap().unwrap().character.name, "Second");
        saves.delete(0).expect("deletes");
        assert!(saves.read(0).is_none());
        saves.delete(0).expect("deleting an empty slot is no error");
    }

    #[test]
    fn a_file_that_is_not_a_save_is_an_error_that_names_it() {
        let saves = saves("broken");
        std::fs::create_dir_all(saves.dir()).unwrap();
        std::fs::write(saves.dir().join("character-3.save"), b"rubbish").unwrap();
        let why = saves.read(2).expect("there").expect_err("refused");
        assert!(why.contains("character-3.save") && why.contains("not a save file"), "{why}");
    }

    #[test]
    fn the_default_folder_is_under_lanterns_save_folder() {
        let dir = Saves::default_dir("explore-the-world");
        assert!(dir.ends_with("save/explore-the-world"), "{}", dir.display());
    }
}
