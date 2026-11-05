//! erfs-nostd-demo - Use `erfs-rt` without `std` or `alloc`.
//!
//! The `erfs-rt` dependency is built with `default-features = false`, so only
//! the borrowed, allocation-free `Fs` API is available. This binary itself
//! uses std for console output, like the other `*-nostd-demo` samples.

use erfs_rt::{Fs, Visit};

mod erfs_gensrc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== erfs-nostd-demo ===");
    println!("erfs-rt built with: default-features = false (no std, no alloc)\n");

    println!("1. Parse the embedded image with the borrowed Fs");
    println!("   ---------------------------------------------");
    let fs = Fs::parse(erfs_gensrc::IMAGE)?;
    println!("   resource id: {}", erfs_gensrc::ID);
    println!("   endian: {:?}", fs.endian());
    println!("   entries: {}", fs.entry_count());
    println!(
        "   /hello.txt: {:?}",
        std::str::from_utf8(fs.read("/hello.txt")?)?
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

    println!("3. Directory iteration without allocation");
    println!("   --------------------------------------");
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

    println!("=== Demo Complete ===");
    Ok(())
}
