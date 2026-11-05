use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use erfs_gen::{Endianness, Options};

const USAGE: &str = "\
erfs-gen - Embedded Resource Filesystem image generator

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
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut positional: Vec<String> = Vec::new();
    let mut little = false;
    let mut big = false;
    let mut rust_binding = true;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--image" => {}
            "--rust" => rust_binding = true,
            "--no-rust" => rust_binding = false,
            "--le" => little = true,
            "--be" => big = true,
            "--both" => {
                little = true;
                big = true;
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("erfs-gen {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}\n\n{USAGE}"));
            }
            other => positional.push(other.to_string()),
        }
    }

    if positional.len() != 3 {
        return Err(format!(
            "expected 3 arguments, got {}\n\n{USAGE}",
            positional.len()
        ));
    }

    let endianness = match (little, big) {
        (false, false) => Endianness::Native,
        (true, false) => Endianness::Little,
        (false, true) => Endianness::Big,
        (true, true) => Endianness::Both,
    };

    let options = Options::new()
        .endianness(endianness)
        .rust_binding(rust_binding);

    let source = PathBuf::from(&positional[0]);
    let id = &positional[1];
    let dest = PathBuf::from(&positional[2]);

    let artifact =
        erfs_gen::generate(&source, id, &dest, &options).map_err(|err| err.to_string())?;

    println!(
        "Packed {} ({} entries, {} bytes)",
        source.display(),
        artifact.entry_count,
        artifact.total_size
    );
    for image in artifact.images() {
        println!("Generated image: {}", image.display());
    }
    if let Some(binding) = &artifact.rust_binding {
        println!("Generated Rust binding: {}", binding.display());
    }

    Ok(())
}
