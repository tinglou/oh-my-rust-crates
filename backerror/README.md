# backerror

`backerror` is a Rust library that enhances error handling by automatically capturing location information and stack traces for errors. It provides a seamless integration with [thiserror](https://github.com/dtolnay/thiserror) to improve debugging capabilities by tracking where errors originate in your code.
When an error is thrown, error chain and stack trace are captured and displayed in a user-friendly format.

## Features

* **Automatic Location Tracking**: Automatically captures the source location (file, line, column) where an error originates
* **Stack Trace Capture**: Optionally captures full stack traces for better debugging
* **Seamless Integration**: Works with existing `thiserror`-based error types
* **Zero-cost Abstraction**: Only incurs runtime cost when enabled (can be disabled in release builds)
* **Transparent Wrapping**: Preserves the original error type while adding location metadata
* **Conditional Compilation**: Can be disabled in release builds to minimize overhead

## Installation

Execute the following command to add `backerror` to your project:

```bash
cargo add backerror
```

Or add this to your `Cargo.toml`:

```toml
[dependencies]
backerror = "0.1"
thiserror = "2.0"
```

## Usage

### Example Codes

```rust
use backerror::backerror;
use thiserror::Error;

#[backerror]
#[derive(Debug, Error)]
#[error(transparent)]
pub struct MyError1(#[from] std::io::Error);

#[backerror]
#[derive(Debug, Error)]
pub enum MyError2 {
    #[error("By MyError2: {0}")]
    My1(#[from] MyError1),
}

#[backerror]
#[derive(Debug, Error)]
pub enum MyError3 {
    #[error("By MyError3: {0}")]
    My2(#[from] MyError2),
}

fn throw_error1() -> Result<(), MyError1> {
    std::fs::File::open("blurb.txt")?;
    Ok(())
}

fn throw_error2() -> Result<(), MyError2> {
    Ok(throw_error1()?)
}
fn throw_error3() -> Result<(), MyError3> {
    Ok(throw_error2()?)
}

#[test]
fn test_display() {
    if let Err(err) = throw_error3() {
        println!("{}", err);
    }
}

#[test]
fn test_debug() {
    if let Err(e) = throw_error3() {
        println!("{:?}", e);
    }
}
```

### Example Output

#### Display Output(`to_string`)

```text
By MyError3: By MyError2: The system cannot find the file specified. (os error 2); Caused by example::MyError2 (backerror\tests\example.rs:32:8); Caused by example::MyError1 (backerror\tests\example.rs:29:8); Caused by std::io::error::Error(backerror\tests\example.rs:24:5);
```

#### Debug Output

```text
My2(My1(MyError1(Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." }
Caused by: example::MyError2: By MyError2: The system cannot find the file specified. (os error 2) (backerror\tests\example.rs:32:8)
        at example::impl$10::from (.\tests\example.rs:16)
        at core::result::impl$28::from_residual<tuple$<>,enum2$<example::MyError2>,enum2$<example::MyError3> > (C:\Users\admin\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\result.rs:2189)
        at example::throw_error3 (.\tests\example.rs:32)
        at example::test_debug (.\tests\example.rs:44)
        at example::test_debug::closure$0 (.\tests\example.rs:43)
        at core::ops::function::FnOnce::call_once<example::test_debug::closure_env$0,tuple$<> > (C:\Users\admin\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250)
        at core::ops::function::FnOnce::call_once (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\core\src\ops\function.rs:250)
        at test::__rust_begin_short_backtrace<enum2$<core::result::Result<tuple$<>,alloc::string::String> >,enum2$<core::result::Result<tuple$<>,alloc::string::String> > (*)()> (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\test\src\lib.rs:663)
        at test::run_test_in_process (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\test\src\lib.rs:686)
        at test::run_test::closure$0 (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\test\src\lib.rs:607)
        at test::run_test::closure$1 (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\test\src\lib.rs:637)
        at std::sys::backtrace::__rust_begin_short_backtrace<test::run_test::closure_env$1,tuple$<> > (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\std\src\sys\backtrace.rs:160)
        at std::thread::lifecycle::spawn_unchecked::closure$1::closure$0 (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\std\src\thread\lifecycle.rs:92)
        at core::panic::unwind_safe::impl$25::call_once (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\core\src\panic\unwind_safe.rs:274)
        at std::panicking::catch_unwind::do_call (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\std\src\panicking.rs:581)
        at std::panicking::catch_unwind (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\std\src\panicking.rs:544)
        at std::panic::catch_unwind (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\std\src\panic.rs:359)
        at std::thread::lifecycle::spawn_unchecked::closure$1 (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\std\src\thread\lifecycle.rs:90)
        at core::ops::function::FnOnce::call_once<std::thread::lifecycle::spawn_unchecked::closure_env$1<test::run_test::closure_env$1,tuple$<> >,tuple$<> > (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\core\src\ops\function.rs:250)
        at std::sys::thread::windows::impl$0::new::thread_start (/rustc/254b59607d4417e9dffbc307138ae5c86280fe4c/library\std\src\sys\thread\windows.rs:59)
        at BaseThreadInitThunk
        at RtlUserThreadStart
Caused by: example::MyError1: The system cannot find the file specified. (os error 2) (backerror\tests\example.rs:29:8)
        at example::impl$5::from (.\tests\example.rs:9)
        at core::result::impl$28::from_residual<tuple$<>,example::MyError1,enum2$<example::MyError2> > (C:\Users\admin\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\result.rs:2189)
        at example::throw_error2 (.\tests\example.rs:29)
Caused by: std::io::error::Error: The system cannot find the file specified. (os error 2) (backerror\tests\example.rs:24:5)
        at example::impl$0::from (.\tests\example.rs:4)
        at core::result::impl$28::from_residual<tuple$<>,std::io::error::Error,example::MyError1> (C:\Users\admin\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\result.rs:2189)
        at example::throw_error1 (.\tests\example.rs:24)
)
)
```

## Rust Features

The crate provides several optional features:

* `backtrace`: Enables backtrace capture only when `RUST_BACKTRACE` environment variable is set
* `force_backtrace`: Forces backtrace capture regardless of environment variables (enabled by default)
* `release_off`: Disables the backerror transformation in release builds (enabled by default)

To customize features:

```toml
[dependencies]
backerror = { version = "...", default-features = false, features = ["force_backtrace"] }
```

## How It Works

The `backerror` crate works by:

1. Providing a `#[backerror]` attribute macro that transforms your error types
2. Converting `#[from] T` attributes to `#[from] LocatedError<T>`
3. Using Rust's `#[track_caller]` attribute to capture the location where errors are converted
4. Optionally capturing a full backtrace when the error is created

When an error occurs, it gets wrapped in a `LocatedError<T>` struct that preserves the original error while adding location metadata.

## License

Apache License, Version 2.0 [LICENSE-APACHE](http://www.apache.org/licenses/LICENSE-2.0)
