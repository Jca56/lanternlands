//! Reading and writing the game's binary files: bytes front to back,
//! little-endian, and tables of names so files hold what things are
//! called rather than what number they happen to have today.

use crate::world::VOID;

/// The names in use and, for each item, its place among them. `None`
/// items (no ground) get [`VOID`].
pub(crate) fn name_table(items: impl Iterator<Item = Option<&'static str>>) -> (Vec<&'static str>, Vec<u8>) {
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

pub(crate) fn write_names(out: &mut Vec<u8>, names: &[&str]) {
    out.push(names.len() as u8);
    for name in names {
        out.push(name.len() as u8);
        out.extend_from_slice(name.as_bytes());
    }
}

/// Text of up to 255 bytes: its length, then the bytes. Longer text is
/// cut, between letters.
pub(crate) fn write_text(out: &mut Vec<u8>, text: &str) {
    let bytes = &text.as_bytes()[..text.floor_char_boundary(255)];
    out.push(bytes.len() as u8);
    out.extend_from_slice(bytes);
}

/// A file's sections as bytes after its magic: version, count, then
/// each as tag, length, payload.
pub(crate) fn write_sections(out: &mut Vec<u8>, sections: &[([u8; 4], Vec<u8>)]) {
    for (tag, payload) in sections {
        out.extend_from_slice(tag);
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
    }
}

/// Bytes read front to back; running out is an error, not a panic.
pub(crate) struct Reader<'a> {
    pub(crate) bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if n > self.bytes.len() {
            return Err("the file ends early".to_owned());
        }
        let (head, rest) = self.bytes.split_at(n);
        self.bytes = rest;
        Ok(head)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("two bytes")))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("four bytes")))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("eight bytes")))
    }

    pub(crate) fn i32(&mut self) -> Result<i32, String> {
        Ok(self.u32()? as i32)
    }

    /// Text written by [`write_text`].
    pub(crate) fn text(&mut self, what: &str) -> Result<String, String> {
        let length = self.u8()? as usize;
        let text = std::str::from_utf8(self.take(length)?).map_err(|_| format!("{what} is not text"))?;
        Ok(text.to_owned())
    }

    pub(crate) fn tag(&mut self) -> Result<[u8; 4], String> {
        Ok(self.take(4)?.try_into().expect("four bytes"))
    }

    /// A name table, each name turned into what it names by `find`.
    pub(crate) fn names<T>(&mut self, what: &str, find: impl Fn(&str) -> Option<T>) -> Result<Vec<T>, String> {
        (0..self.u8()?)
            .map(|_| {
                let length = self.u8()? as usize;
                let name = std::str::from_utf8(self.take(length)?).map_err(|_| format!("a {what} name is not text"))?;
                find(name).ok_or(format!("this build has no {what} called {name:?}"))
            })
            .collect()
    }

    pub(crate) fn done(&self, what: &str) -> Result<(), String> {
        if self.bytes.is_empty() { Ok(()) } else { Err(format!("{} bytes left over after {what}", self.bytes.len())) }
    }
}
