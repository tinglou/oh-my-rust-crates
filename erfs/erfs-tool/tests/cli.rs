use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use erfs_gen::{Endian, build_image};

const ROOT_TREE: &str = "\
Name                   Type  Size
/                      D        3
├── empty              D        0
├── hello.txt          F       11
└── sub                D        2
    ├── deeper         D        1
    │   └── bytes.bin  F      256
    └── nested.txt     F        7
";

const ROOT_LS: &str = "\
Name       Type  Size
empty      D        0
hello.txt  F       11
sub        D        2
";

const SUB_LS: &str = "\
Name        Type  Size
deeper      D        1
nested.txt  F        7
";

struct Fixture {
    dir: PathBuf,
    le_image: PathBuf,
    be_image: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("erfs-tool-it-{}-{nanos}", std::process::id()));
        let resources = dir.join("resources");
        fs::create_dir_all(resources.join("empty")).unwrap();
        fs::create_dir_all(resources.join("sub/deeper")).unwrap();
        fs::write(resources.join("hello.txt"), b"hello erfs\n").unwrap();
        fs::write(resources.join("sub/nested.txt"), b"nested\n").unwrap();
        fs::write(
            resources.join("sub/deeper/bytes.bin"),
            (0u8..=255).collect::<Vec<u8>>(),
        )
        .unwrap();

        let le_image = dir.join("erfs_le_test.img");
        let be_image = dir.join("erfs_be_test.img");
        fs::write(&le_image, build_image(&resources, Endian::Little).unwrap()).unwrap();
        fs::write(&be_image, build_image(&resources, Endian::Big).unwrap()).unwrap();

        Fixture {
            dir,
            le_image,
            be_image,
        }
    }

    fn image(&self, endian: Endian) -> &Path {
        match endian {
            Endian::Little => &self.le_image,
            Endian::Big => &self.be_image,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn tool(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_erfs-tool"))
        .args(args)
        .output()
        .unwrap()
}

fn image_arg(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}

#[test]
fn tree_prints_the_same_output_for_both_endians() {
    let fixture = Fixture::new();

    for endian in [Endian::Little, Endian::Big] {
        let output = tool(&["tree", "/", image_arg(fixture.image(endian))]);
        assert!(output.status.success(), "{}", text(&output.stderr));
        assert_eq!(text(&output.stdout), ROOT_TREE, "endian {endian:?}");
    }
}

#[test]
fn tree_accepts_path_variants_and_prints_the_entry_name() {
    let fixture = Fixture::new();
    let image = image_arg(fixture.image(Endian::Little));

    let expected = "\
Name               Type  Size
sub                D        2
├── deeper         D        1
│   └── bytes.bin  F      256
└── nested.txt     F        7
";

    for path in ["sub", "/sub", "sub/", "//sub//"] {
        let output = tool(&["tree", path, image]);
        assert!(output.status.success(), "{}", text(&output.stderr));
        assert_eq!(text(&output.stdout), expected, "path {path:?}");
    }
}

#[test]
fn ls_lists_direct_children_only() {
    let fixture = Fixture::new();
    let image = image_arg(fixture.image(Endian::Little));

    let output = tool(&["ls", "/", image]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), ROOT_LS);

    for path in ["sub", "/sub", "sub/", "//sub//"] {
        let output = tool(&["ls", path, image]);
        assert!(output.status.success(), "{}", text(&output.stderr));
        assert_eq!(text(&output.stdout), SUB_LS, "path {path:?}");
    }

    let output = tool(&["ls", "/empty", image]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "Name  Type  Size\n");
}

#[test]
fn cat_writes_raw_file_content() {
    let fixture = Fixture::new();
    let image = image_arg(fixture.image(Endian::Big));

    for (path, expected) in [
        ("/hello.txt", b"hello erfs\n".as_slice()),
        ("hello.txt", b"hello erfs\n".as_slice()),
        ("/sub/nested.txt", b"nested\n".as_slice()),
    ] {
        let output = tool(&["cat", path, image]);
        assert!(output.status.success(), "{}", text(&output.stderr));
        assert_eq!(output.stdout, expected, "path {path:?}");
    }

    let expected: Vec<u8> = (0u8..=255).collect();
    let output = tool(&["cat", "/sub/deeper/bytes.bin", image]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(output.stdout, expected);
}

#[test]
fn cat_c_style_dump_escapes_bytes() {
    let fixture = Fixture::new();
    let resources = fixture.dir.join("dump-resources");
    fs::create_dir_all(&resources).unwrap();
    fs::write(resources.join("mixed.bin"), b"Hi\n\t\x00\x1f\x7f\x80").unwrap();
    let image = fixture.dir.join("dump.img");
    fs::write(&image, build_image(&resources, Endian::Little).unwrap()).unwrap();

    let output = tool(&["cat", "-c", "/mixed.bin", image_arg(&image)]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "00000000  H  i \\n \\t \\0 1f 7f 80\n"
    );
}

#[test]
fn cat_hex_dump_uses_offsets_and_spaces() {
    let fixture = Fixture::new();
    let resources = fixture.dir.join("dump-resources");
    fs::create_dir_all(&resources).unwrap();
    fs::write(resources.join("mixed.bin"), b"0123456789abcdefXYZ").unwrap();
    let image = fixture.dir.join("dump.img");
    fs::write(&image, build_image(&resources, Endian::Big).unwrap()).unwrap();

    let output = tool(&["cat", "-x", "/mixed.bin", image_arg(&image)]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "\
00000000 30 31 32 33 34 35 36 37 38 39 61 62 63 64 65 66
00000010 58 59 5a
"
    );
}

#[test]
fn cat_dump_options_conflict() {
    let fixture = Fixture::new();
    let image = image_arg(fixture.image(Endian::Little));

    let output = tool(&["cat", "-c", "-x", "/hello.txt", image]);
    assert!(!output.status.success());
}

#[test]
fn cat_short_h_prints_help() {
    let output = tool(&["cat", "-h"]);
    assert!(output.status.success());
    assert!(text(&output.stdout).contains("Usage: erfs-tool cat"));
    assert!(text(&output.stdout).contains("-x, --hex"));
}

#[test]
fn tree_dirs_only_ignores_files() {
    let fixture = Fixture::new();
    let image = image_arg(fixture.image(Endian::Little));

    let expected = "\
Name            Type  Size
/               D        3
├── empty       D        0
└── sub         D        2
    └── deeper  D        1
";

    for args in [
        vec!["tree", "-d", "/", image],
        vec!["tree", "--dirs-only", "/", image],
    ] {
        let output = tool(&args);
        assert!(output.status.success(), "{}", text(&output.stderr));
        assert_eq!(text(&output.stdout), expected, "args {args:?}");
    }
}

#[test]
fn commands_reject_wrong_entry_types() {
    let fixture = Fixture::new();
    let image = image_arg(fixture.image(Endian::Little));

    let output = tool(&["ls", "/hello.txt", image]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("is not a directory"));

    let output = tool(&["tree", "/hello.txt", image]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("is not a directory"));

    let output = tool(&["cat", "/sub", image]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("cannot read '/sub'"));
}

#[test]
fn reports_missing_and_invalid_images() {
    let fixture = Fixture::new();

    let missing = fixture.dir.join("missing.img");
    let output = tool(&["ls", "/", image_arg(&missing)]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("cannot open file"));

    let invalid = fixture.dir.join("invalid.img");
    fs::write(&invalid, b"not an erfs image").unwrap();
    let output = tool(&["tree", "/", image_arg(&invalid)]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("invalid ERFS image"));
}

#[test]
fn reports_missing_paths_and_unknown_commands() {
    let fixture = Fixture::new();
    let image = image_arg(fixture.image(Endian::Little));

    for command in ["ls", "cat", "tree"] {
        let output = tool(&[command, "/does-not-exist", image]);
        assert_eq!(output.status.code(), Some(1));
        assert!(text(&output.stderr).contains("path not found"));
    }

    let output = tool(&["unknown", "/", image]);
    assert!(!output.status.success());
}

#[test]
fn prints_help_and_version() {
    let output = tool(&["--help"]);
    assert!(output.status.success());
    let help = text(&output.stdout);
    assert!(help.contains("Usage: erfs-tool"));
    assert!(help.contains("ls"));
    assert!(help.contains("cat"));
    assert!(help.contains("tree"));

    let output = tool(&["--version"]);
    assert!(output.status.success());
    assert!(text(&output.stdout).starts_with("erfs-tool "));
}
