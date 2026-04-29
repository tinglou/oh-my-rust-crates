# backerror-macros

This crate provides the procedural macros for the [`backerror`](https://crates.io/crates/backerror) library.

## Overview

`backerror-macros` is a proc-macro crate that powers the `#[backerror]` attribute used in the `backerror` library. It automatically transforms `thiserror`-based error types to capture location information and stack traces.

## What It Does

The `#[backerror]` macro:

1. **Transforms error types**: Converts `#[from] T` attributes to `#[from] backerror::LocatedError<T>`
2. **Generates `From` implementations**: Creates `From<T>` implementations with `#[track_caller]` for location tracking
3. **Works with enums and transparent structs**: Supports both enum error types and `#[error(transparent)]` struct wrappers

## Usage

This crate is typically used indirectly through the `backerror` crate. See the main [`backerror`](https://crates.io/crates/backerror) documentation for usage examples.

```rust
use backerror::backerror;
use thiserror::Error;

#[backerror]
#[derive(Debug, Error)]
pub enum MyError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
}
```

## Features

- `release_off`: Disables the `#[backerror]` transformation in release builds, making it a no-op for zero overhead in production.

## License

Apache License, Version 2.0 [LICENSE-APACHE](http://www.apache.org/licenses/LICENSE-2.0)