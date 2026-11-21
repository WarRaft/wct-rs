//! Reader and writer for Warcraft III custom script files,
//! `war3map.wct`: the code World Editor's trigger editor keeps as text,
//! the map's custom script (its header) and each trigger converted to
//! custom text.
//!
//! ```no_run
//! let bytes = std::fs::read("war3map.wct").unwrap();
//! let file = wct_rs::parse(&bytes).unwrap();
//! println!("{}", String::from_utf8_lossy(file.header.text()));
//! assert_eq!(file.to_bytes().unwrap(), bytes);
//! ```
//!
//! Reading and writing are exact: `to_bytes` gives back the bytes `parse`
//! read. Code is kept as bytes, in whatever encoding the map was saved
//! with. The format is described in the README. For people and for the
//! language server, a file has a text form ([`text`]): the code, one
//! piece after another.

pub mod text;

use parser_rs::bin::{Reader, Writer};
pub use parser_rs::{BinError, BinResult};

/// What a file from 1.31 on starts with.
pub const MAGIC: u32 = 0x8000_0004;

/// One piece of code: what a trigger's custom text holds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Code(
    /// The bytes as stored, the terminating NUL included; `None` when the
    /// file stores no string at all (a size of 0).
    pub Option<Vec<u8>>,
);

impl Code {
    /// The code without its terminating NUL.
    pub fn text(&self) -> &[u8] {
        match &self.0 {
            Some(b) => b.strip_suffix(&[0]).unwrap_or(b),
            None => &[],
        }
    }

    /// Code holding `text`, NUL-terminated as World Editor stores it.
    pub fn new(text: &[u8]) -> Code {
        let mut bytes = text.to_vec();
        bytes.push(0);
        Code(Some(bytes))
    }
}

/// A whole `war3map.wct`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomText {
    /// Whether it starts with [`MAGIC`] (1.31 and later).
    pub magic: bool,
    /// 0 for Reign of Chaos, 1 from The Frozen Throne: only 1 has the
    /// header.
    pub version: u32,
    /// The comment on the map's custom script (version 1), as stored,
    /// without its NUL: in the map's encoding.
    pub comment: Vec<u8>,
    /// The map's custom script (version 1): code before every trigger's.
    pub header: Code,
    /// Each trigger's code, in the trigger editor's order. Before 1.31
    /// every trigger has one (empty unless it's custom text); from 1.31
    /// only triggers that aren't comments do, which `war3map.wtg` says.
    pub triggers: Vec<Code>,
}

impl CustomText {
    /// The file's bytes.
    pub fn to_bytes(&self) -> BinResult<Vec<u8>> {
        let mut w = Writer::new();
        if self.magic {
            w.u32(MAGIC);
        }
        w.u32(self.version);
        if self.version == 1 {
            if let Some(i) = self.comment.iter().position(|&b| b == 0) {
                return Err(BinError::new(w.len() + i, "the comment contains a NUL"));
            }
            w.bytes(&self.comment);
            w.u8(0);
            write_code(&mut w, &self.header)?;
        }
        if !self.magic {
            w.count(self.triggers.len())?;
        }
        for code in &self.triggers {
            write_code(&mut w, code)?;
        }
        Ok(w.into_bytes())
    }
}

fn write_code(w: &mut Writer, code: &Code) -> BinResult<()> {
    match &code.0 {
        None => w.u32(0),
        Some(bytes) => {
            w.count(bytes.len())?;
            w.bytes(bytes);
        }
    }
    Ok(())
}

fn read_code(r: &mut Reader) -> BinResult<Code> {
    let at = r.pos();
    let size = r.u32()? as usize;
    if size == 0 {
        return Ok(Code(None));
    }
    if size > r.remaining() {
        return Err(BinError::new(
            at,
            format!("code of {size} bytes, {} left", r.remaining()),
        ));
    }
    Ok(Code(Some(r.bytes(size)?.to_vec())))
}

/// A NUL-terminated string's bytes, in any encoding.
fn read_cstr(r: &mut Reader) -> BinResult<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        match r.u8()? {
            0 => return Ok(out),
            b => out.push(b),
        }
    }
}

/// Reads a `war3map.wct`. Every byte must be read. From 1.31 the file
/// doesn't say how many triggers it has: they run to its end.
pub fn parse(bytes: &[u8]) -> BinResult<CustomText> {
    let mut r = Reader::new(bytes);
    let mut first = r.u32()?;
    let magic = first == MAGIC;
    if magic {
        first = r.u32()?;
    }
    if first > 1 {
        return Err(BinError::new(
            if magic { 4 } else { 0 },
            format!("unknown wct version {first} (0 and 1 are known)"),
        ));
    }
    let mut file = CustomText {
        magic,
        version: first,
        ..CustomText::default()
    };
    if first == 1 {
        file.comment = read_cstr(&mut r)?;
        file.header = read_code(&mut r)?;
    }
    if magic {
        while !r.is_empty() {
            file.triggers.push(read_code(&mut r)?);
        }
    } else {
        file.triggers = r.list(4, read_code)?;
    }
    r.finish()?;
    Ok(file)
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod tests;
