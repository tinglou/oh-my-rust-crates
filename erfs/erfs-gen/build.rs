extern crate cc;
use std::env;

fn build_cpp_gen() {
    let src = [
        "src/erfs_generator.cpp",
    //    "src/gzip_file.cpp",
    ];
    let mut builder = cc::Build::new();
    let build = builder
        .cpp(true) // Switch to C++ library compilation.
        .flag("-std=c++17")     // enable c++17
        .files(src.iter())
        .include("src")
        ;
    build.compile("rfs_gen_cpp");  
}

#[allow(dead_code)]
fn generate_rust_binding() {
    use bindgen::builder;
    {
        // Configure and generate bindings.
        let bindings = builder().header("src/erfs_generator.h").generate().unwrap();
        // Write the generated bindings to an output file.
        let mut outfile = env::var("OUT_DIR").unwrap();
        outfile += "/erfs_gen_binding.rs";
        bindings.write_to_file(outfile).unwrap();
    }
}

fn main() {
        /*
    pkg_config::Config::new()
        .atleast_version("1.2")
        .probe("z")
        .unwrap();
        */
    build_cpp_gen();

    // commented for crates.io, because src directory is read-only
    generate_rust_binding();
}