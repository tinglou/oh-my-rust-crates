
extern crate cc;
use std::env;

fn generate_rfs_ut() {
    use erfs_gen::erfs_generate;
    {
        erfs_generate("../erfs-gen", "gensrc", 6, &env::var("OUT_DIR").unwrap());
    }
}

fn compile_rfs_source() {
    let mut file = env::var("OUT_DIR").unwrap();
    file += "/erfs_gensrc.c";
    let src = [
        &file,
    ];
    let mut builder = cc::Build::new();
    let build = builder
        .files(src.iter())
        .include(&env::var("OUT_DIR").unwrap())
        .include("../erfs-rt/src")
        ;
    build.compile("rfs_gensrc");  
}

fn main() {
    // generate ERFS source files
    generate_rfs_ut();
    // compile ERFS c source files
    compile_rfs_source();
}