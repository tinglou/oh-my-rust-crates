//! Patch for `thiserror`
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

// Always: reexport macro

pub use backerror_macros::backerror;

// Enable: debug_assertions or not(release_off)

#[cfg(any(not(feature = "release_off"), debug_assertions))]
mod located_error;

#[cfg(any(not(feature = "release_off"), debug_assertions))]
#[cfg(feature = "backtrace")]
mod stacktrace;

#[cfg(any(not(feature = "release_off"), debug_assertions))]
pub use located_error::LocatedError;


// Disable: otherwise
#[cfg(not(any(not(feature = "release_off"), debug_assertions)))]
mod located_error_off;

#[cfg(not(any(not(feature = "release_off"), debug_assertions)))]
pub use located_error_off::LocatedErrorOff as LocatedError;