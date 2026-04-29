#[cfg(feature = "backtrace")]
use std::borrow::Cow;
use std::ops::Deref;
use std::panic::Location;
#[cfg(feature = "backtrace")]
use std::sync::Arc;

/// New error type encapsulating the original error and location data.
/// ```ignore
/// fn open_fail() -> Result<(), LocatedError<std::io::Error>> {
///     std::fs::File::open("blurb.txt")?;
///     Ok(())
/// }
/// let _r = open_fail();
/// ```
pub struct LocatedError<E: std::error::Error> {
    inner: E,
    location: &'static Location<'static>,

    #[cfg(feature = "backtrace")]
    backtrace: Arc<std::backtrace::Backtrace>,
}

/// Error
impl<E: std::error::Error> std::error::Error for LocatedError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

const DEBUG_CAUSED_BY_PAT: &str = "Caused by: ";
const DISPLAY_CAUSED_BY_PAT: &str = "; Caused by ";

/// Display
impl<E: std::error::Error> std::fmt::Display for LocatedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner_msg = format!("{}", self.inner);
        if let Some(pos) = inner_msg.find(DISPLAY_CAUSED_BY_PAT) {
            write!(
                f,
                "{}{DISPLAY_CAUSED_BY_PAT}{} ({}){}",
                &inner_msg[..pos],
                std::any::type_name::<E>(),
                self.location,
                &inner_msg[pos..]
            )
        } else {
            write!(
                f,
                "{}{DISPLAY_CAUSED_BY_PAT}{}({});",
                self.inner,
                std::any::type_name::<E>(),
                self.location,
            )
        }
    }
}

/// Debug
impl<E: std::error::Error> std::fmt::Debug for LocatedError<E> {
    #[cfg(not(feature = "backtrace"))]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let name = std::any::type_name::<E>();
        // let pos = name.rfind(":").unwrap_or(0);
        // let name = &name[pos + 1..];
        write!(
            f,
            "{:?}\n\tat ({}) by {}",
            self.inner,
            self.location,
            std::any::type_name::<E>(), // name
        )
    }

    #[cfg(feature = "backtrace")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(stacktrace) = super::stacktrace::StackTrace::parse(&self.backtrace) {
            self.fmt_stacktrace(stacktrace, f)
        } else {
            write!(
                f,
                "{:?}\n\tat ({}) by {}",
                self.inner,
                self.location,
                std::any::type_name::<E>() // name
            )
        }
    }
}

#[cfg(feature = "backtrace")]
impl<E: std::error::Error> LocatedError<E> {
    fn fmt_stacktrace(
        &self,
        stacktrace: super::stacktrace::StackTrace,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let mut output = Vec::new();
        let inner_debug = format!("{:?}", self.inner);
        let mut lines = inner_debug.lines();

        let mut first_caused_by = true;
        while let Some(line) = lines.next() {
            if first_caused_by {
                if line.starts_with(DEBUG_CAUSED_BY_PAT) {
                    first_caused_by = false;
                    // inject the stacktrace
                    self.inject_stacktrace(&stacktrace, &mut output);
                }
                output.push(Cow::Borrowed(line));
            } else {
                let line = Cow::Borrowed(line);
                if !output.contains(&line) {
                    output.push(line);
                }
            }
        }
        if first_caused_by {
            // inject the stacktrace
            self.inject_stacktrace(&stacktrace, &mut output);
        }

        for line in output {
            writeln!(f, "{}", line)?;
        }

        write!(f, "")
    }

    #[cfg(feature = "backtrace")]
    fn inject_stacktrace(
        &self,
        stacktrace: &crate::stacktrace::StackTrace,
        output: &mut Vec<Cow<'_, str>>,
    ) {
        let cause = format!(
            "{DEBUG_CAUSED_BY_PAT}{}: {} ({})",
            std::any::type_name::<E>(),
            self.pure_desc(),
            self.location
        );
        output.push(Cow::Owned(cause));
        for frame in &stacktrace.frames {
            let trace = if frame.file.is_empty() {
                format!("\tat {}", frame.func)
            } else {
                format!("\tat {} ({}:{})", frame.func, frame.file, frame.line)
            };
            output.push(Cow::Owned(trace));
        }
    }

    fn pure_desc(&self) -> String {
        let desc = self.inner.to_string();
        let desc = if let Some(pos) = desc.find(DISPLAY_CAUSED_BY_PAT) {
            desc[..pos].to_string()
        } else {
            desc
        };
        desc
    }
}

/// From
impl<E: std::error::Error> From<E> for LocatedError<E> {
    #[track_caller]
    fn from(err: E) -> Self {
        LocatedError {
            inner: err,
            location: std::panic::Location::caller(),

            #[cfg(all(feature = "backtrace", not(feature = "force_backtrace")))]
            backtrace: Arc::new(std::backtrace::Backtrace::capture()),

            #[cfg(feature = "force_backtrace")]
            backtrace: Arc::new(std::backtrace::Backtrace::force_capture()), // or Backtrace::disabled()
        }
    }
}

/// AsRef
impl<T: std::error::Error> AsRef<T> for LocatedError<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

/// Deref
impl<T: std::error::Error> Deref for LocatedError<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

/// Borrow
impl<T: std::error::Error> std::borrow::Borrow<T> for LocatedError<T> {
    fn borrow(&self) -> &T {
        &self.inner
    }
}

// Send
unsafe impl<T: std::error::Error + Send> Send for LocatedError<T> {}

/// Sync
unsafe impl<T: std::error::Error + Sync> Sync for LocatedError<T> {}

// Clone
impl<T: std::error::Error + Clone> Clone for LocatedError<T> {
    fn clone(&self) -> Self {
        LocatedError {
            inner: self.inner.clone(),
            location: self.location,

            #[cfg(feature = "backtrace")]
            backtrace: self.backtrace.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum MyError {
        #[error("MyError {0}")]
        LocatedIoError(#[from] LocatedError<std::io::Error>),
    }

    impl From<std::io::Error> for MyError {
        #[track_caller]
        fn from(err: std::io::Error) -> Self {
            MyError::from(LocatedError::from(err))
        }
    }

    fn located_error1() -> Result<(), MyError> {
        std::fs::File::open("blurb.txt").map_err(|e| LocatedError::<std::io::Error>::from(e))?;
        Ok(())
    }

    fn located_error2() -> Result<(), MyError> {
        std::fs::File::open("blurb.txt")?;
        Ok(())
    }

    use super::*;
    #[test]
    fn test_located_error() {
        if let Err(e) = located_error1() {
            println!("==========================================================");
            println!("{}", e);
            println!("==========================================================");
            println!("{:?}", e);
        }

        if let Err(e) = located_error2() {
            println!("==========================================================");
            println!("{}", e);
            println!("==========================================================");
            println!("{:?}", e);
        }
    }
}
