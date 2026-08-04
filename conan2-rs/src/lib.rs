//！# conan-rs: A Rust library for integrating conan into rust custom build scripts.
//! ##  References
//! - [Conan Reference](https://docs.conan.io/2/reference.html)
//! - [Json schema](https://docs.conan.io/2/reference/commands/formatters/graph_info_json_formatter.html)
//! - [Conan source code](https://github.com/conan-io/conan)

pub mod command;
pub mod model;

/// Conan binary override environment variable
const CONAN_ENV: &str = "CONAN";

/// Default Conan binary name
const DEFAULT_CONAN: &str = "conan";

/// Conan output file
const CONAN_OUT_FILE: &'static str = "conanbuildinfo.json";

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
