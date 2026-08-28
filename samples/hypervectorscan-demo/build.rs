fn main() {
    link_stdcpp();
}

/// Link stdc++: dynamic in debug, static in release.
///
/// Note: `cargo:rustc-link-arg` flags are placed AFTER rustc's own `-lstdc++` in the
/// linker command, so the `-Wl,-Bstatic` approach cannot override rustc's dynamic linking.
/// Use `cargo:rustc-link-lib=static=stdc++` instead — rustc handles the `static=` prefix
/// by wrapping the library in `-Bstatic`/`-Bdynamic` at the correct position.
fn link_stdcpp() {
    #[cfg(target_os = "linux")]
    {
        #[cfg(debug_assertions)]
        println!("cargo:rustc-link-lib=stdc++");

        #[cfg(not(debug_assertions))]
        println!("cargo:rustc-link-lib=static=stdc++");
    }
}
