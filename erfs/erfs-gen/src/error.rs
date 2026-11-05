use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Errors produced by `erfs-gen`.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An I/O operation failed.
    Io { path: PathBuf, source: io::Error },
    /// The source path does not exist.
    NotFound(PathBuf),
    /// The source path is unusable (e.g. it has no file name).
    InvalidSource(PathBuf),
    /// The output directory does not exist or is not a directory.
    TargetNotFound(PathBuf),
    /// The id is not a valid identifier (`[a-z][a-z0-9_]*`).
    InvalidId(String),
    /// A path cannot be represented in generated Rust source.
    NonUtf8Path(PathBuf),
    /// The image would contain more than 65535 entries.
    TooManyEntries(usize),
    /// An entry name is longer than 65535 bytes.
    NameTooLong(PathBuf),
    /// A file is larger than 4 GiB.
    EntryTooLarge(PathBuf),
    /// The image is larger than 4 GiB.
    ImageTooLarge(usize),
}

impl Error {
    pub(crate) fn io(path: &Path, source: io::Error) -> Self {
        Error::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Error::NotFound(path) => write!(f, "source not found: {}", path.display()),
            Error::InvalidSource(path) => write!(f, "invalid source: {}", path.display()),
            Error::TargetNotFound(path) => {
                write!(f, "target directory not found: {}", path.display())
            }
            Error::InvalidId(id) => write!(f, "invalid id {id:?}: expected [a-z][a-z0-9_]*"),
            Error::NonUtf8Path(path) => write!(f, "path is not valid UTF-8: {}", path.display()),
            Error::TooManyEntries(count) => {
                write!(f, "too many entries: {count} (max 65535)")
            }
            Error::NameTooLong(path) => {
                write!(f, "entry name too long: {}", path.display())
            }
            Error::EntryTooLarge(path) => write!(f, "file too large: {}", path.display()),
            Error::ImageTooLarge(size) => write!(f, "image too large: {size} bytes"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
