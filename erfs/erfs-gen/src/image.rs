use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::Error;

/// Size of the image header in bytes.
pub const HEADER_SIZE: usize = 12;
/// Size of a single entry in bytes.
pub const ENTRY_SIZE: usize = 16;
/// Magic value written in the requested byte order.
pub const MAGIC: u32 = 0x4552_4653;
/// Entry flag: the entry is a directory.
pub const FLAG_DIRECTORY: u16 = 1;

/// Byte order of a generated image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    /// The byte order of the host.
    pub fn native() -> Self {
        if cfg!(target_endian = "big") {
            Endian::Big
        } else {
            Endian::Little
        }
    }

    /// The file name infix, e.g. `le` or `be`.
    pub fn short_name(self) -> &'static str {
        match self {
            Endian::Little => "le",
            Endian::Big => "be",
        }
    }

    fn push_u16(self, out: &mut Vec<u8>, value: u16) {
        match self {
            Endian::Little => out.extend_from_slice(&value.to_le_bytes()),
            Endian::Big => out.extend_from_slice(&value.to_be_bytes()),
        }
    }

    fn push_u32(self, out: &mut Vec<u8>, value: u32) {
        match self {
            Endian::Little => out.extend_from_slice(&value.to_le_bytes()),
            Endian::Big => out.extend_from_slice(&value.to_be_bytes()),
        }
    }
}

struct Node {
    name: Vec<u8>,
    path: PathBuf,
    is_dir: bool,
    children: Vec<Node>,
}

struct FlatEntry {
    name: Vec<u8>,
    path: PathBuf,
    is_dir: bool,
    data: Vec<u8>,
    data_offset: u32,
    data_size: u32,
    name_offset: u32,
    name_size: u16,
    flags: u16,
}

impl FlatEntry {
    fn new(name: Vec<u8>, path: PathBuf, is_dir: bool) -> Self {
        FlatEntry {
            name,
            path,
            is_dir,
            data: Vec::new(),
            data_offset: 0,
            data_size: 0,
            name_offset: 0,
            name_size: 0,
            flags: if is_dir { FLAG_DIRECTORY } else { 0 },
        }
    }
}

/// Builds an image from a directory (or a single file) in memory.
pub fn build_image(source: &Path, endian: Endian) -> Result<Vec<u8>, Error> {
    let root = scan(source)?;
    let mut entries = Vec::new();
    entries.push(FlatEntry::new(root.name.clone(), root.path.clone(), true));
    flatten(&root, 0, &mut entries);
    serialize(&mut entries, endian)
}

fn scan(source: &Path) -> Result<Node, Error> {
    let metadata = fs::metadata(source).map_err(|err| {
        if err.kind() == io::ErrorKind::NotFound {
            Error::NotFound(source.to_path_buf())
        } else {
            Error::io(source, err)
        }
    })?;

    if metadata.is_dir() {
        let mut root = Node {
            name: b"/".to_vec(),
            path: source.to_path_buf(),
            is_dir: true,
            children: Vec::new(),
        };
        scan_dir(source, &mut root.children)?;
        Ok(root)
    } else {
        let name = source
            .file_name()
            .map(|name| name.as_encoded_bytes().to_vec())
            .filter(|name| !name.is_empty())
            .ok_or_else(|| Error::InvalidSource(source.to_path_buf()))?;
        let file = Node {
            name,
            path: source.to_path_buf(),
            is_dir: false,
            children: Vec::new(),
        };
        Ok(Node {
            name: b"/".to_vec(),
            path: source.parent().unwrap_or(source).to_path_buf(),
            is_dir: true,
            children: vec![file],
        })
    }
}

fn scan_dir(dir: &Path, out: &mut Vec<Node>) -> Result<(), Error> {
    let read_dir = fs::read_dir(dir).map_err(|err| Error::io(dir, err))?;
    for entry in read_dir {
        let entry = entry.map_err(|err| Error::io(dir, err))?;
        let path = entry.path();
        let metadata = fs::metadata(&path).map_err(|err| Error::io(&path, err))?;
        let name = entry.file_name().as_encoded_bytes().to_vec();

        if metadata.is_dir() {
            let mut node = Node {
                name,
                path: path.clone(),
                is_dir: true,
                children: Vec::new(),
            };
            scan_dir(&path, &mut node.children)?;
            out.push(node);
        } else {
            out.push(Node {
                name,
                path,
                is_dir: false,
                children: Vec::new(),
            });
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(())
}

fn flatten(dir: &Node, ordinal: usize, entries: &mut Vec<FlatEntry>) {
    let first_child = entries.len();
    for child in &dir.children {
        entries.push(FlatEntry::new(
            child.name.clone(),
            child.path.clone(),
            child.is_dir,
        ));
    }

    entries[ordinal].data_offset = (first_child * ENTRY_SIZE) as u32;
    entries[ordinal].data_size = dir.children.len() as u32;

    for (index, child) in dir.children.iter().enumerate() {
        if child.is_dir {
            flatten(child, first_child + index, entries);
        }
    }
}

fn serialize(entries: &mut [FlatEntry], endian: Endian) -> Result<Vec<u8>, Error> {
    let count = entries.len();
    if count > u16::MAX as usize {
        return Err(Error::TooManyEntries(count));
    }

    let table_size = count * ENTRY_SIZE;

    let mut name_cursor = table_size;
    for entry in entries.iter_mut() {
        if entry.name.len() > u16::MAX as usize {
            return Err(Error::NameTooLong(entry.path.clone()));
        }
        entry.name_offset = name_cursor as u32;
        entry.name_size = entry.name.len() as u16;
        name_cursor += entry.name.len();
    }

    let mut cursor = name_cursor;
    for entry in entries.iter_mut() {
        if entry.is_dir {
            continue;
        }
        let data = fs::read(&entry.path).map_err(|err| Error::io(&entry.path, err))?;
        if data.len() > u32::MAX as usize {
            return Err(Error::EntryTooLarge(entry.path.clone()));
        }
        entry.data_size = data.len() as u32;
        entry.data_offset = cursor as u32;
        cursor += data.len();
        entry.data = data;
    }

    let total_size = HEADER_SIZE + cursor;
    if total_size > u32::MAX as usize {
        return Err(Error::ImageTooLarge(total_size));
    }

    let mut out = Vec::with_capacity(total_size);
    endian.push_u32(&mut out, MAGIC);
    endian.push_u32(&mut out, total_size as u32);
    endian.push_u16(&mut out, count as u16);
    endian.push_u16(&mut out, 0);

    for entry in entries.iter() {
        endian.push_u32(&mut out, entry.data_offset);
        endian.push_u32(&mut out, entry.data_size);
        endian.push_u32(&mut out, entry.name_offset);
        endian.push_u16(&mut out, entry.name_size);
        endian.push_u16(&mut out, entry.flags);
    }

    for entry in entries.iter() {
        out.extend_from_slice(&entry.name);
    }

    for entry in entries.iter() {
        if !entry.is_dir {
            out.extend_from_slice(&entry.data);
        }
    }

    debug_assert_eq!(out.len(), total_size);
    Ok(out)
}
