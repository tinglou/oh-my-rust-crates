//! Example of embedding an ERFS image into a Rust crate.
//!
//! `build.rs` packs `assets/` with `erfs-gen` into `$OUT_DIR`, and this crate
//! embeds the resulting images with `include_bytes!` (see
//! [generated binding][erfs_gensrc]) and reads them with `erfs-rt`.

use erfs_rt::Fs;

pub mod erfs_gensrc;

/// Resource id used by `build.rs`.
pub const ID: &str = erfs_gensrc::ID;

/// Filesystem over the native-endian image.
pub fn fs() -> Fs<'static> {
    Fs::parse(erfs_gensrc::IMAGE).expect("invalid embedded ERFS image")
}

/// Filesystem over the little-endian image.
pub fn fs_le() -> Fs<'static> {
    Fs::parse(erfs_gensrc::IMAGE_LE).expect("invalid embedded ERFS image")
}

/// Filesystem over the big-endian image.
pub fn fs_be() -> Fs<'static> {
    Fs::parse(erfs_gensrc::IMAGE_BE).expect("invalid embedded ERFS image")
}

/// Filesystem embedded directly with the `erfs_rt::fs!` macro.
pub fn fs_macro() -> Fs<'static> {
    erfs_rt::fs!(concat!(env!("OUT_DIR"), "/erfs_le_gensrc.img"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use erfs_rt::{Error, Visit};

    #[test]
    fn read_file() {
        assert_eq!(fs().read("/hello.txt").unwrap(), b"Hello, ERFS!\n");
        assert_eq!(
            fs().read("/data/config.json").unwrap(),
            include_bytes!("../assets/data/config.json")
        );
    }

    #[test]
    fn open_and_inspect() {
        let fs = fs();
        let entry = fs.open("/data").unwrap();
        assert!(entry.is_dir());
        assert_eq!(entry.name(), b"data");
        assert_eq!(entry.size(), 2);

        let names: Vec<&str> = entry
            .children()
            .unwrap()
            .map(|child| child.name_str().unwrap())
            .collect();
        assert_eq!(names, ["config.json", "notes.md"]);
    }

    #[test]
    fn walk_files() {
        let fs = fs();
        let mut paths = Vec::new();
        fs.walk(|visit| {
            if let Visit::File(entry) = visit {
                paths.push(entry.name_str().unwrap());
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(paths, ["config.json", "notes.md", "hello.txt"]);
    }

    #[test]
    fn both_endians_read_the_same() {
        let le = fs_le();
        let be = fs_be();
        assert_eq!(le.endian(), erfs_rt::Endian::Little);
        assert_eq!(be.endian(), erfs_rt::Endian::Big);
        for path in ["/hello.txt", "/data/config.json", "/data/notes.md"] {
            assert_eq!(le.read(path).unwrap(), be.read(path).unwrap());
        }
    }

    #[test]
    fn missing_path() {
        assert_eq!(fs().read("/nope.txt"), Err(Error::NotFound));
    }

    #[test]
    fn embed_with_macro() {
        assert_eq!(fs_macro().read("/hello.txt").unwrap(), b"Hello, ERFS!\n");
    }
}
