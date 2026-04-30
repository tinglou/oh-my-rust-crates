//! Patch for `thiserror`
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

mod located_error;

#[cfg(feature = "backtrace")]
mod stacktrace;

pub use backerror_macros::backerror;
pub use located_error::LocatedError;
