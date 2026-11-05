//! erfs-demo - Embed and read an ERFS image with `std` and `alloc` enabled.
//!
//! `build.rs` packs `assets/` with `erfs-gen` into `$OUT_DIR`; this binary
//! embeds the generated images with `include_bytes!` (through the generated
//! binding) and reads them with `erfs-rt`'s default features (`std` + `alloc`).

use erfs_rt::{Fs, OwnedFs, Visit};

mod erfs_gensrc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== erfs-demo ===");
    println!("erfs-rt built with: default features (std + alloc)\n");

    println!("1. Read files from the generated binding");
    println!("   ------------------------------------");
    let fs = Fs::parse(erfs_gensrc::IMAGE)?;
    println!("   resource id: {}", erfs_gensrc::ID);
    println!("   endian: {:?}", fs.endian());
    println!("   entries: {}", fs.entry_count());
    println!(
        "   /hello.txt: {:?}",
        std::str::from_utf8(fs.read("/hello.txt")?)?
    );
    println!(
        "   /data/config.json: {:?}",
        std::str::from_utf8(fs.read("/data/config.json")?)?
    );
    println!();

    println!("2. Little-endian and big-endian images");
    println!("   ------------------------------------");
    let le = Fs::parse(erfs_gensrc::IMAGE_LE)?;
    let be = Fs::parse(erfs_gensrc::IMAGE_BE)?;
    println!("   IMAGE_LE endian: {:?}", le.endian());
    println!("   IMAGE_BE endian: {:?}", be.endian());
    for path in ["/hello.txt", "/data/config.json", "/data/notes.md"] {
        assert_eq!(le.read(path)?, be.read(path)?, "endian mismatch for {path}");
    }
    println!("   both images expose identical content");
    println!();

    println!("3. Inspect a directory");
    println!("   --------------------");
    let data = fs.open("/data")?;
    println!(
        "   /data is dir: {}, children: {}",
        data.is_dir(),
        data.size()
    );
    for child in data.children()? {
        println!("     {} ({} bytes)", child.name_str()?, child.size());
    }
    println!();

    println!("4. Depth-first walk");
    println!("   -----------------");
    fs.walk(|visit| {
        if let Visit::File(entry) = visit {
            println!("   file: {} ({} bytes)", entry.name_str()?, entry.size());
        }
        Ok(())
    })?;
    println!();

    println!("5. erfs_rt::fs! macro");
    println!("   ------------------");
    let fs_macro = erfs_rt::fs!(concat!(env!("OUT_DIR"), "/erfs_le_gensrc.img"));
    println!(
        "   /hello.txt: {:?}",
        std::str::from_utf8(fs_macro.read("/hello.txt")?)?
    );
    println!();

    println!("6. Owned mode (requires alloc/std)");
    println!("   -------------------------------");
    let image_path = concat!(env!("OUT_DIR"), "/erfs_le_gensrc.img");
    let owned = OwnedFs::from_file(image_path)?;
    println!("   loaded {} bytes from OUT_DIR", owned.as_bytes().len());
    println!(
        "   /data/notes.md: {} bytes",
        owned.read("/data/notes.md")?.len()
    );

    let copied = OwnedFs::from_slice(erfs_gensrc::IMAGE)?;
    assert_eq!(copied.as_bytes(), erfs_gensrc::IMAGE);
    println!("   OwnedFs::from_slice round-trips the embedded image");
    println!();

    println!("=== Demo Complete ===");
    Ok(())
}
