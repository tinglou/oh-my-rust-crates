use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use erfs_gen::{Endian, Options};

fn reference(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../erfs-c/build/test_gen")
        .join(name);
    path.exists().then_some(path)
}

fn erfs_c_source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../erfs-c/src");
    path.is_dir().then_some(path)
}

#[test]
fn matches_erfs_c_reference_images() {
    let Some(source) = erfs_c_source() else {
        eprintln!("skipping: ../erfs-c/src not found");
        return;
    };

    for (endian, name) in [
        (Endian::Little, "erfs_le_test.img"),
        (Endian::Big, "erfs_be_test.img"),
    ] {
        let Some(reference) = reference(name) else {
            eprintln!("skipping: reference image {name} not found");
            continue;
        };

        let generated = erfs_gen::build_image(&source, endian).unwrap();
        let expected = fs::read(&reference).unwrap();
        assert_eq!(
            generated, expected,
            "generated {name} differs from erfs-c reference"
        );
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("erfs-gen-it-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        TempDir { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn resource_dir() -> TempDir {
    let dir = TempDir::new();
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("a.txt"), b"aaa").unwrap();
    fs::write(dir.path().join("sub/b.txt"), b"bbb").unwrap();
    dir
}

#[test]
fn writes_requested_artifacts() {
    let source = resource_dir();
    let out = TempDir::new();

    let options = Options::new().both_endians().rust_binding(true);
    let artifact = erfs_gen::generate(source.path(), "app", out.path(), &options).unwrap();

    let le = out.path().join("erfs_le_app.img");
    let be = out.path().join("erfs_be_app.img");
    let rs = out.path().join("erfs_app.rs");

    assert_eq!(artifact.le_image.as_deref(), Some(le.as_path()));
    assert_eq!(artifact.be_image.as_deref(), Some(be.as_path()));
    assert_eq!(artifact.rust_binding.as_deref(), Some(rs.as_path()));
    assert_eq!(artifact.image(), Some(le.as_path()));
    assert_eq!(artifact.images().count(), 2);
    assert_eq!(artifact.entry_count, 4);

    let le_bytes = fs::read(&le).unwrap();
    let be_bytes = fs::read(&be).unwrap();
    assert!(artifact.total_size > 0);
    assert_eq!(le_bytes.len(), be_bytes.len());
    assert_eq!(le_bytes.len(), fs::metadata(&le).unwrap().len() as usize);

    let binding = fs::read_to_string(&rs).unwrap();
    assert!(binding.contains("include_bytes!("));
    assert!(binding.contains("IMAGE_LE"));
    assert!(binding.contains("IMAGE_BE"));
    assert!(binding.contains("IMAGE"));
}

#[test]
fn output_is_deterministic() {
    let source = resource_dir();
    let out = TempDir::new();

    let options = Options::new().little_endian().rust_binding(false);
    erfs_gen::generate(source.path(), "app", out.path(), &options).unwrap();
    let first = fs::read(out.path().join("erfs_le_app.img")).unwrap();

    erfs_gen::generate(source.path(), "app", out.path(), &options).unwrap();
    let second = fs::read(out.path().join("erfs_le_app.img")).unwrap();

    assert_eq!(first, second);
}

#[test]
fn packs_a_single_file() {
    let source = resource_dir();
    let file = source.path().join("a.txt");

    let image = erfs_gen::build_image(&file, Endian::Little).unwrap();

    assert_eq!(&image[..4], b"SFRE");
    assert_eq!(
        u32::from_le_bytes(image[4..8].try_into().unwrap()) as usize,
        image.len()
    );
    assert_eq!(u16::from_le_bytes(image[8..10].try_into().unwrap()), 2);

    let name_offset = u32::from_le_bytes(image[12 + 16 + 8..12 + 16 + 12].try_into().unwrap());
    let name_size = u16::from_le_bytes(image[12 + 16 + 12..12 + 16 + 14].try_into().unwrap());
    let name = &image[12 + name_offset as usize..12 + name_offset as usize + name_size as usize];
    assert_eq!(name, b"a.txt");
}

#[test]
fn sorts_entries_by_name() {
    let source = TempDir::new();
    for name in ["z.txt", "a.txt", "m.txt"] {
        fs::write(source.path().join(name), b"x").unwrap();
    }

    let image = erfs_gen::build_image(source.path(), Endian::Little).unwrap();
    let root_name_offset = u32::from_le_bytes(image[12 + 8..12 + 12].try_into().unwrap());
    let root_name_size = u16::from_le_bytes(image[12 + 12..12 + 14].try_into().unwrap());
    let root_name = &image
        [12 + root_name_offset as usize..12 + root_name_offset as usize + root_name_size as usize];
    assert_eq!(root_name, b"/");

    let mut names = Vec::new();
    for index in 1..4 {
        let entry = 12 + index * 16;
        let offset = u32::from_le_bytes(image[entry + 8..entry + 12].try_into().unwrap());
        let size = u16::from_le_bytes(image[entry + 12..entry + 14].try_into().unwrap());
        names.push(
            String::from_utf8(
                image[12 + offset as usize..12 + offset as usize + size as usize].to_vec(),
            )
            .unwrap(),
        );
    }
    assert_eq!(names, ["a.txt", "m.txt", "z.txt"]);
}

#[test]
fn validates_id_and_paths() {
    let source = resource_dir();
    let out = TempDir::new();
    let options = Options::new();

    for invalid in ["", "1app", "App", "app-1", "app.x"] {
        assert!(
            erfs_gen::generate(source.path(), invalid, out.path(), &options).is_err(),
            "id {invalid:?} should be rejected"
        );
    }

    for valid in ["app", "a", "app_1", "a0_b"] {
        erfs_gen::validate_id(valid).unwrap();
    }

    let missing = out.path().join("does-not-exist");
    assert!(erfs_gen::generate(&missing, "app", out.path(), &options).is_err());
    assert!(erfs_gen::generate(source.path(), "app", &missing, &options).is_err());
}
