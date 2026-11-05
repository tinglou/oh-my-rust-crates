use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use erfs_gen::Endian;
use erfs_rt::{Endian as ImageEndian, Error, Fs, Visit};

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("erfs-rt-it-{}-{nanos}", std::process::id()));
        fs::create_dir_all(dir.join("empty")).unwrap();
        fs::create_dir_all(dir.join("sub/deeper")).unwrap();
        fs::write(dir.join("hello.txt"), b"hello erfs\n").unwrap();
        fs::write(dir.join("sub/nested.txt"), b"nested\n").unwrap();
        fs::write(
            dir.join("sub/deeper/bytes.bin"),
            (0u8..=255).collect::<Vec<u8>>(),
        )
        .unwrap();
        Fixture { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }

    fn image(&self, endian: Endian) -> Vec<u8> {
        erfs_gen::build_image(self.path(), endian).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn detects_endian_and_reads_files() {
    let fixture = Fixture::new();

    for (endian, expected) in [
        (Endian::Little, ImageEndian::Little),
        (Endian::Big, ImageEndian::Big),
    ] {
        let image = fixture.image(endian);
        let fs = Fs::parse(&image).unwrap();

        assert_eq!(fs.endian(), expected);
        assert_eq!(fs.entry_count(), 7);
        assert_eq!(fs.read("/hello.txt").unwrap(), b"hello erfs\n");
        assert_eq!(fs.read("/sub/nested.txt").unwrap(), b"nested\n");
        assert_eq!(fs.read("/sub/deeper/bytes.bin").unwrap().len(), 256);
    }
}

#[test]
fn accepts_path_variants() {
    let fixture = Fixture::new();
    let image = fixture.image(Endian::Little);
    let fs = Fs::parse(&image).unwrap();

    for path in [
        "/hello.txt",
        "hello.txt",
        "//hello.txt",
        "/hello.txt/",
        "/sub//nested.txt",
    ] {
        let expected = if path.contains("nested") {
            b"nested\n".as_slice()
        } else {
            b"hello erfs\n".as_slice()
        };
        assert_eq!(fs.read(path).unwrap(), expected, "path {path:?}");
    }

    assert_eq!(fs.open("/").unwrap().name(), b"/");
    assert_eq!(fs.open("").unwrap().name(), b"/");
    assert_eq!(fs.open("/sub/").unwrap().name(), b"sub");
}

#[test]
fn lists_directory_in_sorted_order() {
    let fixture = Fixture::new();
    let image = fixture.image(Endian::Little);
    let fs = Fs::parse(&image).unwrap();

    let root = fs.root();
    assert!(root.is_dir());
    assert_eq!(root.size(), 3);

    let names: Vec<&str> = root
        .children()
        .unwrap()
        .map(|entry| entry.name_str().unwrap())
        .collect();
    assert_eq!(names, ["empty", "hello.txt", "sub"]);

    let sub = fs.open("/sub").unwrap();
    let names: Vec<&str> = sub
        .children()
        .unwrap()
        .map(|entry| entry.name_str().unwrap())
        .collect();
    assert_eq!(names, ["deeper", "nested.txt"]);

    assert_eq!(fs.open("/empty").unwrap().size(), 0);
}

#[test]
fn walk_emits_depth_first_events() {
    let fixture = Fixture::new();
    let image = fixture.image(Endian::Big);
    let fs = Fs::parse(&image).unwrap();

    let mut events = Vec::new();
    fs.walk(|visit| {
        let (kind, entry) = match visit {
            Visit::Enter(entry) => ("enter", entry),
            Visit::Leave(entry) => ("leave", entry),
            Visit::File(entry) => ("file", entry),
        };
        events.push(format!("{kind}:{}", entry.name_str().unwrap()));
        Ok(())
    })
    .unwrap();

    assert_eq!(
        events,
        [
            "enter:/",
            "enter:empty",
            "leave:empty",
            "file:hello.txt",
            "enter:sub",
            "enter:deeper",
            "file:bytes.bin",
            "leave:deeper",
            "file:nested.txt",
            "leave:sub",
            "leave:/",
        ]
    );
}

#[test]
fn walk_can_be_aborted() {
    let fixture = Fixture::new();
    let image = fixture.image(Endian::Little);
    let fs = Fs::parse(&image).unwrap();

    let mut seen = 0;
    let result = fs.walk(|_| {
        seen += 1;
        Err(Error::NotFound)
    });

    assert_eq!(result, Err(Error::NotFound));
    assert_eq!(seen, 1);
}

#[test]
fn reports_access_errors() {
    let fixture = Fixture::new();
    let image = fixture.image(Endian::Little);
    let fs = Fs::parse(&image).unwrap();

    assert_eq!(fs.read("/missing"), Err(Error::NotFound));
    assert_eq!(fs.read("/sub"), Err(Error::NotFile));
    assert_eq!(fs.open("/hello.txt/child"), Err(Error::NotDirectory));
    assert!(matches!(
        fs.open("/hello.txt").unwrap().children(),
        Err(Error::NotDirectory)
    ));
    assert_eq!(
        fs.open("/hello.txt").unwrap().get("x"),
        Err(Error::NotDirectory)
    );
    assert_eq!(fs.entry_at(100), Err(Error::OutOfBounds));
}

#[test]
fn rejects_malformed_images() {
    let fixture = Fixture::new();
    let image = fixture.image(Endian::Little);

    assert_eq!(Fs::parse(&image[..4]).unwrap_err(), Error::InvalidSize);

    let mut bad_magic = image.clone();
    bad_magic[0] = b'X';
    assert_eq!(Fs::parse(&bad_magic).unwrap_err(), Error::InvalidMagic);

    let mut bad_size = image.clone();
    bad_size[4] = bad_size[4].wrapping_add(1);
    assert_eq!(Fs::parse(&bad_size).unwrap_err(), Error::InvalidSize);

    let mut bad_offset = image.clone();
    bad_offset[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(Fs::parse(&bad_offset).unwrap_err(), Error::CorruptImage);

    let mut no_entries = image.clone();
    no_entries[8..10].copy_from_slice(&0u16.to_le_bytes());
    assert_eq!(Fs::parse(&no_entries).unwrap_err(), Error::CorruptImage);
}

#[test]
fn gzipped_flag_is_exposed() {
    let fixture = Fixture::new();
    let mut image = fixture.image(Endian::Little);
    let fs = Fs::parse(&image).unwrap();
    let entry = fs.open("/hello.txt").unwrap();
    assert!(!entry.is_gzipped());
    let index = entry.index() as usize;
    let flags_offset = 12 + index * 16 + 14;
    image[flags_offset..flags_offset + 2].copy_from_slice(&erfs_rt::FLAG_GZIPPED.to_le_bytes());
    let fs = Fs::parse(&image).unwrap();
    assert!(fs.open("/hello.txt").unwrap().is_gzipped());
}
