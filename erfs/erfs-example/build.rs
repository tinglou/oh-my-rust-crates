use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let assets = manifest_dir.join("assets");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", assets.display());

    let options = erfs_gen::Options::new().both_endians().rust_binding(true);

    if let Err(err) = erfs_gen::generate(&assets, "gensrc", &out_dir, &options) {
        panic!("erfs-gen failed: {err}");
    }
}
