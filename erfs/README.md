English | [简体中文](README-cn.md)

# Embedded Resource Filesystem (ERFS)

ERFS packs a directory tree into a compact, read-only image and lets an
application read those resources straight from its own binary, without a
filesystem and without copying data.

This workspace contains the pure-Rust implementation:

| Crate                       | Description                                                        |
| --------------------------- | ------------------------------------------------------------------ |
| [`erfs-rt`](erfs-rt)        | `no_std` runtime with borrowed (`&[u8]`) and owned (`Vec<u8>`) modes. |
| [`erfs-gen`](erfs-gen)      | Image generator: library for `build.rs` plus the `erfs-gen` CLI.   |
| [`erfs-example`](erfs-example) | Complete example of embedding and reading an image.             |

The C implementation in [`erfs-c`](erfs-c) is kept as the reference for the
binary format; images generated here are byte-for-byte identical to the ones
produced by `erfs-c` (see [`erfs-c/layout.md`](erfs-c/layout.md)).

## How it works

```
 resources/ ──erfs-gen──▶ erfs_le_app.img ──include_bytes!──▶ erfs-rt::Fs
```

1. `erfs-gen` scans the resource directory and writes an ERFS image.
2. The image is embedded with `include_bytes!` (directly or via the generated
   Rust binding).
3. `erfs-rt` validates the image once and provides zero-copy access.

Both little-endian and big-endian images are supported; the byte order is
detected from the magic, so an image works on any target.

## Command line

```text
Usage: erfs-gen [options] <src_dir> <id> <dest_dir>

Arguments:
  <src_dir>   directory (or single file) to embed
  <id>        resource id, matching [a-z][a-z0-9_]*
  <dest_dir>  existing directory receiving the generated files

Options:
  --image        generate a binary image (default)
  --rust         generate a Rust binding source (default)
  --no-rust      do not generate the Rust binding source
  --le           generate a little-endian image
  --be           generate a big-endian image
  --both         generate both little-endian and big-endian images
  -h, --help     print this help
  -V, --version  print the version
```

Generated files use the same names as `erfs-c`:

* `erfs_le_<id>.img`, `erfs_be_<id>.img` — binary images.
* `erfs_<id>.rs` — Rust binding exposing `IMAGE`, `IMAGE_LE`, `IMAGE_BE`
  and `ID` constants that use `include_bytes!`.

## Using the library from `build.rs`

```rust,no_run
// build.rs
use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let options = erfs_gen::Options::new()
        .both_endians()
        .rust_binding(true);

    erfs_gen::generate("resources", "app", &out_dir, &options).unwrap();
    println!("cargo:rerun-if-changed=resources");
}
```

Then embed the generated binding:

```rust,ignore
pub mod erfs_gensrc {
    include!(concat!(env!("OUT_DIR"), "/erfs_app.rs"));
}
```

## Runtime API

```rust,ignore
use erfs_rt::{Fs, Visit};

const IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/erfs_le_app.img"));

let fs = Fs::parse(IMAGE)?;

// zero-copy file access
let data: &[u8] = fs.read("/dir/file.txt")?;

// open and inspect an entry
let entry = fs.open("/dir")?;
for child in entry.children()? {
    println!("{} ({} bytes)", child.name_str()?, child.size());
}

// depth-first traversal
fs.walk(|visit| {
    if let Visit::File(entry) = visit {
        println!("{}", entry.name_str()?);
    }
    Ok(())
})?;
```

The `erfs_rt::fs!` macro combines embedding and parsing:

```rust,ignore
let fs = erfs_rt::fs!(concat!(env!("OUT_DIR"), "/erfs_le_app.img"));
```

### Owned mode

When the image is loaded at runtime instead of embedded, `OwnedFs` stores it in
a `Vec<u8>` and validates it once; it exposes the same lookup API and shares the
implementation with `Fs` through `OwnedFs::as_fs`:

```rust,no_run
use erfs_rt::OwnedFs;

let fs = OwnedFs::from_file("resources.img")?;
let data: &[u8] = fs.read("/dir/file.txt")?;
```

`OwnedFs::from_bytes` takes a `Vec<u8>`, `from_slice` copies a slice, and
`into_bytes` returns the owned image.

`erfs-rt` is always `#![no_std]`. The borrowed `Fs` mode never allocates; the
owned mode requires the `alloc` feature, and `from_file` requires `std` (both on
by default). For bare-metal targets use `default-features = false`.

## Examples and tests

```sh
cargo test --workspace
cargo run -p erfs-gen -- --both resources myres /tmp/out
```

`erfs-rt` and `erfs-gen` test suites include a golden test that compares the
generated images against the `erfs-c` reference images, when they are present.

## TODO

* symbolic/hard link support

## License

MIT OR Apache-2.0
