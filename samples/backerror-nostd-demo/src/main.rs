//! backerror-nostd-demo - Demonstrates backerror with no_std library
//!
//! This demo shows:
//! - backerror compiled without std (default-features = false)
//! - Basic error types with location tracking (#[track_caller])
//! - Works with std for console output
//!
//! Note: This binary uses std for console output,
//! but demonstrates backerror's no_std compatibility.

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

#[backerror]
#[derive(Debug, Error)]
pub enum IoError {
    #[error("IO Error: {0}")]
    StdIoError(#[from] std::io::Error),
}

fn throw_simple_error() -> Result<(), SimpleError> {
    Err(SimpleError::AnError)
}

fn throw_io_error() -> Result<(), IoError> {
    std::fs::File::open("nonexistent.txt")?;
    Ok(())
}

fn main() {
    println!("=== backerror-nostd-demo ===");
    println!("backerror built with: default-features = false (no std)\n");

    // Demo 1: Simple error
    println!("1. Simple Error (no_std compatible)");
    println!("   --------------------------------");
    match throw_simple_error() {
        Ok(_) => println!("   Success"),
        Err(e) => {
            println!("   Error: {}", e);
            println!("   Debug: {:?}", e);
        }
    }
    println!();

    // Demo 2: IO error with location tracking
    println!("2. IO Error with Location (no_std compatible)");
    println!("   --------------------------------");
    match throw_io_error() {
        Ok(_) => println!("   Success"),
        Err(e) => {
            println!("   Error: {}", e);
            println!("   Debug: {:?}", e);
        }
    }
    println!();

    println!("=== Demo Complete ===");
}
