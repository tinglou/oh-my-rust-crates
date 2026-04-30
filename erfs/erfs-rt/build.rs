extern crate cc;
use std::env;

fn build_c_rt() {
    let src = [
        "src/resource_fs.c",
    ];
    let mut builder = cc::Build::new();
    let build = builder
        .files(src.iter())
        .include("src")
        ;
    build.compile("erfs_c_rt");  
}

#[allow(dead_code)]
fn generate_rust_binding() {
    use bindgen::builder;
    {
        // Configure and generate bindings.
        let bindings = builder().header("src/resource_fs.h").generate().unwrap();
        // Write the generated bindings to an output file.
        let mut outfile = env::var("OUT_DIR").unwrap();
        outfile += "/erfs_binding.rs";
        bindings.write_to_file(outfile).unwrap();
    }
}


fn main() {
    build_c_rt();

    // commented for crates.io, because src directory is read-only
    generate_rust_binding();
}