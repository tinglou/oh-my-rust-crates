pub mod install;
mod types;

use std::path::PathBuf;

pub use types::*;

use crate::{CONAN_ENV, DEFAULT_CONAN};

pub fn find_conan_program() -> Option<PathBuf> {
    if let Ok(conan) = std::env::var(CONAN_ENV) {
        return Some(PathBuf::from(conan));
    }
    which::which(DEFAULT_CONAN).ok()
}
