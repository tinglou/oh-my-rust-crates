use core::error::Error;
use core::ops::Deref;
use core::{borrow, fmt};

/// New error type encapsulating the original error and location data.
/// ```ignore
/// fn open_fail() -> Result<(), LocatedErrorOff<std::io::Error>> {
///     std::fs::File::open("blurb.txt")?;
///     Ok(())
/// }
/// let _r = open_fail();
/// ```
#[repr(transparent)]
pub struct LocatedErrorOff<E: Error>(E);

/// Error
impl<E: Error> Error for LocatedErrorOff<E> {
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

/// Display
impl<E: Error> fmt::Display for LocatedErrorOff<E> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Debug
impl<E: Error> fmt::Debug for LocatedErrorOff<E> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

/// From
impl<E: Error> From<E> for LocatedErrorOff<E> {
    #[inline]
    fn from(err: E) -> Self {
        LocatedErrorOff(err)
    }
}

/// AsRef
impl<T: Error> AsRef<T> for LocatedErrorOff<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0
    }
}

/// Deref
impl<T: Error> Deref for LocatedErrorOff<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

/// Borrow
impl<T: Error> borrow::Borrow<T> for LocatedErrorOff<T> {
    #[inline]
    fn borrow(&self) -> &T {
        &self.0
    }
}

// Send
unsafe impl<T: Error + Send> Send for LocatedErrorOff<T> {}

/// Sync
unsafe impl<T: Error + Sync> Sync for LocatedErrorOff<T> {}

// Clone
impl<T: Error + Clone> Clone for LocatedErrorOff<T> {
    #[inline]
    fn clone(&self) -> Self {
        LocatedErrorOff(self.0.clone())
    }
}
