#![no_std]

use backerror::backerror;
use thiserror::Error;

#[backerror]
#[derive(Debug, Error)]
pub enum SimpleError {
    #[error("An error occurred")]
    AnError,
    #[error("Another error occurred")]
    AnotherError,
}

fn throw_error() -> Result<(), SimpleError> {
    Err(SimpleError::AnError)
}

fn main() {
    // Simple demonstration that works in no_std environment
    if let Err(err) = throw_error() {
        // In no_std, we just demonstrate the type works
        let _ = err;
    }
}
