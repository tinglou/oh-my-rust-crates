//! Read-only runtime for ERFS (Embedded Resource Filesystem) images.
//!
//! `erfs-rt` is `#![no_std]`, has no dependencies and performs no allocation.
//! An image produced by `erfs-gen` is embedded with [`include_bytes!`] and
//! parsed with [`Fs::parse`]:
//!
//! ```ignore
//! use erfs_rt::Fs;
//!
//! const IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/erfs_le_app.img"));
//!
//! let fs = Fs::parse(IMAGE).unwrap();
//! let hello = fs.read("/hello.txt").unwrap();
//! ```
//!
//! The [`fs!`] macro combines both steps:
//!
//! ```ignore
//! let fs = erfs_rt::fs!(concat!(env!("OUT_DIR"), "/erfs_le_app.img"));
//! ```
//!
//! Both little-endian and big-endian images are supported; the byte order is
//! detected from the image magic.
//!
//! # Storage modes
//!
//! * [`Fs`] borrows the image (`&[u8]`). This is the `no_std`, allocation-free
//!   mode used together with [`include_bytes!`].
//! * [`OwnedFs`] owns the image (`Vec<u8>`) and can load it from a file. Both
//!   types share the same parsing and lookup code; [`OwnedFs::as_fs`] exposes
//!   the borrowed view.
//!
//! The owned mode is enabled by the `alloc` feature, and loading from a file
//! additionally requires the `std` feature. `std` is on by default; use
//! `default-features = false` for bare-metal targets.

#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

use core::cmp::Ordering;
use core::fmt;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::path::Path;

/// Magic bytes of a little-endian image (`0x4552_4653` written LE).
pub const MAGIC_LE: [u8; 4] = *b"SFRE";
/// Magic bytes of a big-endian image (`0x4552_4653` written BE).
pub const MAGIC_BE: [u8; 4] = *b"ERFS";

/// Size of the image header in bytes.
pub const HEADER_SIZE: usize = 12;
/// Size of a single entry in bytes.
pub const ENTRY_SIZE: usize = 16;

/// Entry flag: the entry is a directory.
pub const FLAG_DIRECTORY: u16 = 1;
/// Entry flag: the entry content is gzip compressed.
pub const FLAG_GZIPPED: u16 = 2;

const ENTRY_SIZE_U32: u32 = ENTRY_SIZE as u32;

/// Byte order of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    #[inline]
    fn u16(self, bytes: &[u8]) -> u16 {
        let bytes = [bytes[0], bytes[1]];
        match self {
            Endian::Little => u16::from_le_bytes(bytes),
            Endian::Big => u16::from_be_bytes(bytes),
        }
    }

    #[inline]
    fn u32(self, bytes: &[u8]) -> u32 {
        let bytes = [bytes[0], bytes[1], bytes[2], bytes[3]];
        match self {
            Endian::Little => u32::from_le_bytes(bytes),
            Endian::Big => u32::from_be_bytes(bytes),
        }
    }
}

/// Errors returned while parsing or accessing an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The image does not start with a known magic.
    InvalidMagic,
    /// The declared total size does not match the buffer length.
    InvalidSize,
    /// The image is structurally invalid (offsets out of bounds).
    CorruptImage,
    /// The requested path or name does not exist.
    NotFound,
    /// The entry is a directory, not a file.
    NotFile,
    /// The entry is a file, not a directory.
    NotDirectory,
    /// An entry index is out of bounds.
    OutOfBounds,
    /// A name is not valid UTF-8.
    InvalidName,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Error::InvalidMagic => "invalid ERFS magic",
            Error::InvalidSize => "image size does not match header",
            Error::CorruptImage => "corrupt ERFS image",
            Error::NotFound => "path not found",
            Error::NotFile => "entry is not a file",
            Error::NotDirectory => "entry is not a directory",
            Error::OutOfBounds => "entry index out of bounds",
            Error::InvalidName => "entry name is not valid UTF-8",
        };
        f.write_str(msg)
    }
}

impl core::error::Error for Error {}

#[derive(Debug, Clone, Copy)]
struct RawEntry {
    data_offset: u32,
    data_size: u32,
    name_offset: u32,
    name_size: u16,
    flags: u16,
}

/// A validated ERFS image.
///
/// `Fs` is a lightweight, `Copy` view over the image bytes. It borrows the
/// image, so it can be stored statically (see [`fs!`]).
#[derive(Clone, Copy)]
pub struct Fs<'a> {
    image: &'a [u8],
    endian: Endian,
    entry_count: u16,
}

impl<'a> Fs<'a> {
    /// Validates `image` and returns a filesystem view.
    pub fn parse(image: &'a [u8]) -> Result<Self, Error> {
        if image.len() < HEADER_SIZE {
            return Err(Error::InvalidSize);
        }

        let endian = match &image[..4] {
            m if m == MAGIC_LE => Endian::Little,
            m if m == MAGIC_BE => Endian::Big,
            _ => return Err(Error::InvalidMagic),
        };

        if endian.u32(&image[4..8]) as usize != image.len() {
            return Err(Error::InvalidSize);
        }

        let entry_count = endian.u16(&image[8..10]);
        if entry_count == 0 {
            return Err(Error::CorruptImage);
        }

        let fs = Fs::from_parts(image, endian, entry_count);
        fs.validate()?;
        Ok(fs)
    }

    /// Builds a view from an image that has already been validated.
    #[inline]
    fn from_parts(image: &'a [u8], endian: Endian, entry_count: u16) -> Self {
        Fs {
            image,
            endian,
            entry_count,
        }
    }

    /// The raw image bytes.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.image
    }

    /// The byte order of the image.
    #[inline]
    pub fn endian(&self) -> Endian {
        self.endian
    }

    /// Number of entries (files and directories, including the root).
    #[inline]
    pub fn entry_count(&self) -> u16 {
        self.entry_count
    }

    /// Returns the entry at `index`.
    pub fn entry_at(&self, index: u16) -> Result<Entry<'a>, Error> {
        self.raw(index)?;
        Ok(Entry { fs: *self, index })
    }

    /// Returns the root directory entry (`/`).
    pub fn root(&self) -> Entry<'a> {
        Entry {
            fs: *self,
            index: 0,
        }
    }

    /// Opens a path such as `/dir/file.txt`.
    ///
    /// A leading `/`, repeated `/` and a trailing `/` are accepted. `.` and
    /// `..` components are not supported.
    pub fn open(&self, path: &str) -> Result<Entry<'a>, Error> {
        let mut entry = self.root();
        for segment in path.split('/') {
            if segment.is_empty() {
                continue;
            }
            entry = entry.get(segment)?;
        }
        Ok(entry)
    }

    /// Reads the content of the file at `path`.
    pub fn read(&self, path: &str) -> Result<&'a [u8], Error> {
        self.open(path)?.data()
    }

    /// Depth-first traversal of the whole filesystem.
    ///
    /// The callback receives [`Visit::Enter`] and [`Visit::Leave`] for
    /// directories and [`Visit::File`] for files. Returning an error aborts
    /// the traversal and is propagated to the caller.
    pub fn walk<F>(&self, mut visit: F) -> Result<(), Error>
    where
        F: FnMut(Visit<'a>) -> Result<(), Error>,
    {
        self.walk_entry(self.root(), &mut visit)
    }

    fn walk_entry<F>(&self, entry: Entry<'a>, visit: &mut F) -> Result<(), Error>
    where
        F: FnMut(Visit<'a>) -> Result<(), Error>,
    {
        if entry.is_dir() {
            visit(Visit::Enter(entry))?;
            for child in entry.children()? {
                self.walk_entry(child, visit)?;
            }
            visit(Visit::Leave(entry))
        } else {
            visit(Visit::File(entry))
        }
    }

    #[inline]
    fn data(&self) -> &'a [u8] {
        &self.image[HEADER_SIZE..]
    }

    fn raw(&self, index: u16) -> Result<RawEntry, Error> {
        if index >= self.entry_count {
            return Err(Error::OutOfBounds);
        }
        let offset = index as usize * ENTRY_SIZE;
        let bytes = &self.data()[offset..offset + ENTRY_SIZE];
        Ok(RawEntry {
            data_offset: self.endian.u32(&bytes[0..4]),
            data_size: self.endian.u32(&bytes[4..8]),
            name_offset: self.endian.u32(&bytes[8..12]),
            name_size: self.endian.u16(&bytes[12..14]),
            flags: self.endian.u16(&bytes[14..16]),
        })
    }

    fn name(&self, raw: &RawEntry) -> &'a [u8] {
        let data = self.data();
        let start = raw.name_offset as usize;
        &data[start..start + raw.name_size as usize]
    }

    fn validate(&self) -> Result<(), Error> {
        let data = self.data();
        let table_len = self.entry_count as usize * ENTRY_SIZE;
        if table_len > data.len() {
            return Err(Error::CorruptImage);
        }

        for index in 0..self.entry_count {
            let raw = self.raw(index)?;

            let name_end = (raw.name_offset as usize)
                .checked_add(raw.name_size as usize)
                .ok_or(Error::CorruptImage)?;
            if name_end > data.len() {
                return Err(Error::CorruptImage);
            }

            if raw.flags & FLAG_DIRECTORY != 0 {
                if raw.data_offset % ENTRY_SIZE_U32 != 0 {
                    return Err(Error::CorruptImage);
                }
                let start = raw.data_offset as usize;
                let count = raw.data_size as usize;
                if count > self.entry_count as usize {
                    return Err(Error::CorruptImage);
                }
                let end = count
                    .checked_mul(ENTRY_SIZE)
                    .and_then(|len| start.checked_add(len))
                    .ok_or(Error::CorruptImage)?;
                if end > table_len {
                    return Err(Error::CorruptImage);
                }
            } else {
                let end = (raw.data_offset as usize)
                    .checked_add(raw.data_size as usize)
                    .ok_or(Error::CorruptImage)?;
                if end > data.len() {
                    return Err(Error::CorruptImage);
                }
            }
        }

        Ok(())
    }
}

impl fmt::Debug for Fs<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fs")
            .field("endian", &self.endian)
            .field("entry_count", &self.entry_count)
            .field("size", &self.image.len())
            .finish()
    }
}

/// A file or directory entry inside an [`Fs`].
#[derive(Clone, Copy)]
pub struct Entry<'a> {
    fs: Fs<'a>,
    index: u16,
}

impl<'a> Entry<'a> {
    /// The filesystem this entry belongs to.
    #[inline]
    pub fn fs(&self) -> Fs<'a> {
        self.fs
    }

    /// The entry index in the entry table.
    #[inline]
    pub fn index(&self) -> u16 {
        self.index
    }

    /// The raw name bytes (not NUL terminated).
    pub fn name(&self) -> &'a [u8] {
        match self.fs.raw(self.index) {
            Ok(raw) => self.fs.name(&raw),
            Err(_) => &[],
        }
    }

    /// The name as UTF-8.
    pub fn name_str(&self) -> Result<&'a str, Error> {
        core::str::from_utf8(self.name()).map_err(|_| Error::InvalidName)
    }

    /// Raw entry flags (see [`FLAG_DIRECTORY`] and [`FLAG_GZIPPED`]).
    pub fn flags(&self) -> u16 {
        self.fs.raw(self.index).map(|raw| raw.flags).unwrap_or(0)
    }

    /// Whether this entry is a directory.
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.flags() & FLAG_DIRECTORY != 0
    }

    /// Whether this entry is a regular file.
    #[inline]
    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }

    /// Whether the content is gzip compressed.
    #[inline]
    pub fn is_gzipped(&self) -> bool {
        self.flags() & FLAG_GZIPPED != 0
    }

    /// File size in bytes, or child count for a directory.
    pub fn size(&self) -> usize {
        self.fs
            .raw(self.index)
            .map(|raw| raw.data_size as usize)
            .unwrap_or(0)
    }

    /// The content of a file. Returns [`Error::NotFile`] for directories.
    pub fn data(&self) -> Result<&'a [u8], Error> {
        if self.is_dir() {
            return Err(Error::NotFile);
        }
        let raw = self.fs.raw(self.index)?;
        let data = self.fs.data();
        let start = raw.data_offset as usize;
        Ok(&data[start..start + raw.data_size as usize])
    }

    /// Looks up a child by name using the sorted entry table.
    pub fn get(&self, name: &str) -> Result<Entry<'a>, Error> {
        if !self.is_dir() {
            return Err(Error::NotDirectory);
        }
        let raw = self.fs.raw(self.index)?;
        let start = (raw.data_offset / ENTRY_SIZE_U32) as u16;
        let count = raw.data_size as u16;
        let target = name.as_bytes();

        let mut low = 0u16;
        let mut high = count;
        while low < high {
            let mid = low + (high - low) / 2;
            let entry = Entry {
                fs: self.fs,
                index: start + mid,
            };
            match entry.name().cmp(target) {
                Ordering::Less => low = mid + 1,
                Ordering::Greater => high = mid,
                Ordering::Equal => return Ok(entry),
            }
        }
        Err(Error::NotFound)
    }

    /// Iterates over the children of a directory.
    pub fn children(&self) -> Result<Children<'a>, Error> {
        if !self.is_dir() {
            return Err(Error::NotDirectory);
        }
        let raw = self.fs.raw(self.index)?;
        Ok(Children {
            fs: self.fs,
            start: (raw.data_offset / ENTRY_SIZE_U32) as u16,
            count: raw.data_size as u16,
            pos: 0,
        })
    }
}

impl fmt::Debug for Entry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field(
                "name",
                &core::str::from_utf8(self.name()).unwrap_or("<invalid>"),
            )
            .field("dir", &self.is_dir())
            .field("size", &self.size())
            .finish()
    }
}

impl PartialEq for Entry<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.fs.image.as_ptr() == other.fs.image.as_ptr()
    }
}

impl Eq for Entry<'_> {}

/// Iterator over the children of a directory.
#[derive(Clone)]
pub struct Children<'a> {
    fs: Fs<'a>,
    start: u16,
    count: u16,
    pos: u16,
}

impl<'a> Iterator for Children<'a> {
    type Item = Entry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.count {
            return None;
        }
        let index = self.start + self.pos;
        self.pos += 1;
        Some(Entry { fs: self.fs, index })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.count - self.pos) as usize;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Children<'_> {}

/// An event emitted by [`Fs::walk`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visit<'a> {
    /// Entering a directory, before its children.
    Enter(Entry<'a>),
    /// Leaving a directory, after its children.
    Leave(Entry<'a>),
    /// Visiting a regular file.
    File(Entry<'a>),
}

impl<'a> Visit<'a> {
    /// The entry associated with the event.
    pub fn entry(self) -> Entry<'a> {
        match self {
            Visit::Enter(entry) | Visit::Leave(entry) | Visit::File(entry) => entry,
        }
    }
}

/// An owning ERFS filesystem backed by a `Vec<u8>`.
///
/// This is the allocated counterpart of [`Fs`]: it takes ownership of the
/// image and validates it once, then serves the same zero-copy lookups through
/// [`OwnedFs::as_fs`]. Use it to load images at runtime, for example from a
/// file with [`OwnedFs::from_file`].
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct OwnedFs {
    image: Vec<u8>,
    endian: Endian,
    entry_count: u16,
}

#[cfg(feature = "alloc")]
impl OwnedFs {
    /// Validates `image` and takes ownership of its bytes.
    pub fn from_bytes(image: Vec<u8>) -> Result<Self, Error> {
        let (endian, entry_count) = {
            let fs = Fs::parse(&image)?;
            (fs.endian(), fs.entry_count())
        };
        Ok(OwnedFs {
            image,
            endian,
            entry_count,
        })
    }

    /// Validates `image` and copies it into a new owning filesystem.
    pub fn from_slice(image: &[u8]) -> Result<Self, Error> {
        Self::from_bytes(image.to_vec())
    }

    /// Loads and validates an image from a file.
    #[cfg(feature = "std")]
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, OpenError> {
        Ok(Self::from_bytes(std::fs::read(path)?)?)
    }

    /// The raw image bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.image
    }

    /// Consumes the filesystem and returns the owned image bytes.
    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.image
    }

    /// A borrowed, already validated view of the same image.
    ///
    /// All lookup helpers delegate to this view, so [`Fs`] and [`OwnedFs`]
    /// share the same implementation.
    #[inline]
    pub fn as_fs(&self) -> Fs<'_> {
        Fs::from_parts(&self.image, self.endian, self.entry_count)
    }

    /// The byte order of the image.
    #[inline]
    pub fn endian(&self) -> Endian {
        self.endian
    }

    /// Number of entries (files and directories, including the root).
    #[inline]
    pub fn entry_count(&self) -> u16 {
        self.entry_count
    }

    /// Returns the entry at `index`.
    pub fn entry_at(&self, index: u16) -> Result<Entry<'_>, Error> {
        self.as_fs().entry_at(index)
    }

    /// Returns the root directory entry (`/`).
    pub fn root(&self) -> Entry<'_> {
        self.as_fs().root()
    }

    /// Opens a path such as `/dir/file.txt`.
    pub fn open(&self, path: &str) -> Result<Entry<'_>, Error> {
        self.as_fs().open(path)
    }

    /// Reads the content of the file at `path`.
    pub fn read(&self, path: &str) -> Result<&[u8], Error> {
        self.as_fs().read(path)
    }

    /// Depth-first traversal of the whole filesystem.
    pub fn walk<'f, F>(&'f self, visit: F) -> Result<(), Error>
    where
        F: FnMut(Visit<'f>) -> Result<(), Error>,
    {
        self.as_fs().walk(visit)
    }
}

#[cfg(feature = "alloc")]
impl fmt::Debug for OwnedFs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedFs")
            .field("endian", &self.endian)
            .field("entry_count", &self.entry_count)
            .field("size", &self.image.len())
            .finish()
    }
}

#[cfg(feature = "alloc")]
impl AsRef<[u8]> for OwnedFs {
    fn as_ref(&self) -> &[u8] {
        &self.image
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<Vec<u8>> for OwnedFs {
    type Error = Error;

    fn try_from(image: Vec<u8>) -> Result<Self, Error> {
        Self::from_bytes(image)
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<&[u8]> for OwnedFs {
    type Error = Error;

    fn try_from(image: &[u8]) -> Result<Self, Error> {
        Self::from_slice(image)
    }
}

/// Error returned by [`OwnedFs::from_file`].
#[cfg(feature = "std")]
#[derive(Debug)]
#[non_exhaustive]
pub enum OpenError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The file content is not a valid ERFS image.
    Image(Error),
}

#[cfg(feature = "std")]
impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Io(err) => err.fmt(f),
            OpenError::Image(err) => err.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::Io(err) => Some(err),
            OpenError::Image(err) => Some(err),
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for OpenError {
    fn from(err: std::io::Error) -> Self {
        OpenError::Io(err)
    }
}

#[cfg(feature = "std")]
impl From<Error> for OpenError {
    fn from(err: Error) -> Self {
        OpenError::Image(err)
    }
}

/// Embeds an image with [`include_bytes!`] and parses it.
///
/// Panics if the image is invalid; prefer [`Fs::parse`] when the image is not
/// trusted.
#[macro_export]
macro_rules! fs {
    ($image:expr $(,)?) => {
        $crate::Fs::parse(include_bytes!($image)).expect("invalid ERFS image")
    };
}
