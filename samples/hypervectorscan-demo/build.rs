fn main() {
    link_stdcpp();
}

/// Link stdc++: dynamic in debug, static in release (including libgcc).
fn link_stdcpp() {
    #[cfg(target_os = "linux")]
    {
        #[cfg(debug_assertions)]
        println!("cargo:rustc-link-lib=stdc++");

        #[cfg(not(debug_assertions))]
        {
            println!("cargo:rustc-link-lib=static=stdc++");
            println!("cargo:rustc-link-lib=static=gcc_eh");
            println!("cargo:rustc-link-lib=static=gcc");
        }
    }
}
