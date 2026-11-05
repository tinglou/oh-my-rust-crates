//! ERFS image generator.
//!
//! `erfs-gen` packs a directory (or a single file) into an ERFS image whose
//! binary layout is identical to the one produced by `erfs-c`. It is available
//! both as a library (for use from `build.rs`) and as the `erfs-gen` command
//! line tool.
//!
//! ```no_run
//! // build.rs
//! let out_dir = std::env::var_os("OUT_DIR").unwrap();
//! let options = erfs_gen::Options::new().both_endians();
//! let artifact = erfs_gen::generate("resources", "app", out_dir, &options).unwrap();
//! println!("cargo:rerun-if-changed=resources");
//! ```

#![forbid(unsafe_code)]

mod binding;
mod error;
mod image;

pub use error::Error;
pub use image::{build_image, Endian, ENTRY_SIZE, FLAG_DIRECTORY, HEADER_SIZE, MAGIC};

use std::path::{Path, PathBuf};

/// Selects which images [`generate`] produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Endianness {
    /// A single image in the host byte order.
    #[default]
    Native,
    /// A little-endian image.
    Little,
    /// A big-endian image.
    Big,
    /// Both little-endian and big-endian images.
    Both,
}

impl Endianness {
    fn flags(self) -> (bool, bool) {
        match self {
            Endianness::Native => match Endian::native() {
                Endian::Little => (true, false),
                Endian::Big => (false, true),
            },
            Endianness::Little => (true, false),
            Endianness::Big => (false, true),
            Endianness::Both => (true, true),
        }
    }
}

/// Options for [`generate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Which byte order(s) to generate. Defaults to [`Endianness::Native`].
    pub endianness: Endianness,
    /// Generate a Rust source file exposing the images via `include_bytes!`.
    /// Defaults to `true`.
    pub rust_binding: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            endianness: Endianness::Native,
            rust_binding: true,
        }
    }
}

impl Options {
    /// Creates the default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the byte order selection.
    pub fn endianness(mut self, endianness: Endianness) -> Self {
        self.endianness = endianness;
        self
    }

    /// Generates a single native-endian image.
    pub fn native_endian(self) -> Self {
        self.endianness(Endianness::Native)
    }

    /// Generates a little-endian image.
    pub fn little_endian(self) -> Self {
        self.endianness(Endianness::Little)
    }

    /// Generates a big-endian image.
    pub fn big_endian(self) -> Self {
        self.endianness(Endianness::Big)
    }

    /// Generates both little-endian and big-endian images.
    pub fn both_endians(self) -> Self {
        self.endianness(Endianness::Both)
    }

    /// Enables or disables the generated Rust binding.
    pub fn rust_binding(mut self, enabled: bool) -> Self {
        self.rust_binding = enabled;
        self
    }
}

/// Files written by [`generate`].
#[derive(Debug, Clone)]
pub struct Artifact {
    /// The resource id.
    pub id: String,
    /// Path of the little-endian image, if generated.
    pub le_image: Option<PathBuf>,
    /// Path of the big-endian image, if generated.
    pub be_image: Option<PathBuf>,
    /// Path of the generated Rust binding, if requested.
    pub rust_binding: Option<PathBuf>,
    /// Size of the primary image in bytes.
    pub total_size: usize,
    /// Number of entries in the primary image.
    pub entry_count: u16,
}

impl Artifact {
    /// The primary image path (little-endian when both were generated).
    pub fn image(&self) -> Option<&Path> {
        self.le_image.as_deref().or(self.be_image.as_deref())
    }

    /// Iterates over all generated image paths.
    pub fn images(&self) -> impl Iterator<Item = &Path> {
        self.le_image
            .iter()
            .chain(self.be_image.iter())
            .map(PathBuf::as_path)
    }
}

/// File name of an image: `erfs_<le|be>_<id>.img`.
pub fn image_file_name(id: &str, endian: Endian) -> String {
    format!("erfs_{}_{}.img", endian.short_name(), id)
}

/// File name of a generated Rust binding: `erfs_<id>.rs`.
pub fn rust_binding_file_name(id: &str) -> String {
    format!("erfs_{id}.rs")
}

/// Checks that `id` matches `[a-z][a-z0-9_]*`.
pub fn validate_id(id: &str) -> Result<(), Error> {
    let mut bytes = id.bytes();
    let valid = matches!(bytes.next(), Some(b'a'..=b'z'))
        && bytes.all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(Error::InvalidId(id.to_string()))
    }
}

/// Packs `source` and writes the requested artifacts into `out_dir`.
pub fn generate(
    source: impl AsRef<Path>,
    id: &str,
    out_dir: impl AsRef<Path>,
    options: &Options,
) -> Result<Artifact, Error> {
    validate_id(id)?;

    let source = source.as_ref();
    let out_dir = out_dir.as_ref();
    if !out_dir.is_dir() {
        return Err(Error::TargetNotFound(out_dir.to_path_buf()));
    }

    let (little, big) = options.endianness.flags();
    let mut artifact = Artifact {
        id: id.to_string(),
        le_image: None,
        be_image: None,
        rust_binding: None,
        total_size: 0,
        entry_count: 0,
    };

    if little {
        let image = build_image(source, Endian::Little)?;
        let path = out_dir.join(image_file_name(id, Endian::Little));
        std::fs::write(&path, &image).map_err(|err| Error::io(&path, err))?;
        if artifact.image().is_none() {
            artifact.total_size = image.len();
            artifact.entry_count = u16::from_le_bytes([image[8], image[9]]);
        }
        artifact.le_image = Some(path);
    }

    if big {
        let image = build_image(source, Endian::Big)?;
        let path = out_dir.join(image_file_name(id, Endian::Big));
        std::fs::write(&path, &image).map_err(|err| Error::io(&path, err))?;
        if artifact.image().is_none() {
            artifact.total_size = image.len();
            artifact.entry_count = u16::from_be_bytes([image[8], image[9]]);
        }
        artifact.be_image = Some(path);
    }

    if options.rust_binding {
        let source = binding::render(
            id,
            artifact.le_image.as_deref(),
            artifact.be_image.as_deref(),
        )?;
        let path = out_dir.join(rust_binding_file_name(id));
        std::fs::write(&path, source).map_err(|err| Error::io(&path, err))?;
        artifact.rust_binding = Some(path);
    }

    Ok(artifact)
}
