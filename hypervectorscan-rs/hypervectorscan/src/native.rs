use std::ffi::{c_int, c_uint, c_ulonglong, c_void};
use std::mem::MaybeUninit;
use std::sync::Arc;

use foreign_types::ForeignType;
use hypervectorscan_sys as hs;

use super::{AsResult, Error, HyperscanErrorCode, Pattern, ScanMode, wrapper};
use crate::SerializedDatabase;

// -------------------------------------------------------------------------------------------------
// Scan Callback
// -------------------------------------------------------------------------------------------------

/// The result returned by a scan callback
///
/// This is also called a "match event handler" in the Vectorscan C API documentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scan {
    Continue,
    Terminate,
}

// -------------------------------------------------------------------------------------------------
// Block Database
// -------------------------------------------------------------------------------------------------

/// A database that supports Vectorscan's block-based matching APIs
#[derive(Clone, Debug)]
pub struct BlockDatabase {
    inner: wrapper::Database,
}

impl BlockDatabase {
    /// Create a new database with the given patterns
    pub fn new(patterns: Vec<Pattern>) -> Result<Self, Error> {
        let inner = wrapper::Database::new(patterns, ScanMode::BLOCK)?;
        Ok(Self { inner })
    }

    /// Create a new scanner from this database
    pub fn create_scanner(self: &Arc<Self>) -> Result<BlockScanner, Error> {
        BlockScanner::new(self.clone())
    }

    /// Get the size in bytes of the database
    pub fn size(&self) -> Result<usize, Error> {
        self.inner.size()
    }

    /// Serializes the database using `hs_serialize_database`.
    pub fn serialize(&self) -> Result<SerializedDatabase, Error> {
        self.inner.serialize()
    }

    /// Deserializes a database using `hs_deserialize_database`.
    pub fn deserialize(sdb: SerializedDatabase) -> Result<Self, Error> {
        let db = wrapper::Database::deserialize(sdb)?;
        Ok(Self { inner: db })
    }
}

// -------------------------------------------------------------------------------------------------
// Block Scanner
// -------------------------------------------------------------------------------------------------

/// A scanner that supports Vectorscan's block-based matching APIs
#[derive(Clone, Debug)]
pub struct BlockScanner {
    scratch: wrapper::Scratch,
    db: Arc<BlockDatabase>,
}

impl BlockScanner {
    /// Create a new scanner with the given database
    pub fn new(db: Arc<BlockDatabase>) -> Result<Self, Error> {
        Ok(Self {
            scratch: wrapper::Scratch::new(&db.inner)?,
            db,
        })
    }

    /// Scan the input using the given callback function
    ///
    /// The callback function takes 4 arguments and returns a `Scan` value.
    /// The 4 arguments:
    ///
    /// - id: u32     The ID of the expression that matched
    /// - from: u64   The offset of the start byte of the match; in practice, always 0
    /// - to: u64     The offset of the byte after the end byte of the match
    /// - flags: u32  Unused; "provided for future use"
    ///
    /// For more detail, see the Hyperscan documentation:
    ///
    /// - [`hs_scan`](https://intel.github.io/hyperscan/dev-reference/api_files.html#c.hs_scan)
    /// - [`match_event_handler`](https://intel.github.io/hyperscan/dev-reference/api_files.html#c.match_event_handler)
    pub fn scan<F>(&mut self, data: &[u8], on_match: F) -> Result<Scan, Error>
    where
        F: FnMut(u32, u64, u64, u32) -> Scan,
    {
        let mut context = Context { on_match };

        let res = unsafe {
            hs::hs_scan(
                self.db.inner.as_ptr(),
                data.as_ptr() as *const _,
                data.len() as u32,
                0,
                self.scratch.as_ptr(),
                Some(on_match_trampoline::<F>),
                &mut context as *mut _ as *mut c_void,
            )
            .ok()
        };

        match res {
            Ok(_) => Ok(Scan::Continue),
            Err(err) => match err {
                Error::Hyperscan(HyperscanErrorCode::ScanTerminated) => Ok(Scan::Terminate),
                err => Err(err),
            },
        }
    }

    /// Get the size in bytes of the scratch space used by this scanner
    pub fn size(&self) -> Result<usize, Error> {
        self.scratch.size()
    }
}

// -------------------------------------------------------------------------------------------------
// Streaming Database
// -------------------------------------------------------------------------------------------------

/// A database that supports Vectorscan's streaming matching APIs
#[derive(Clone, Debug)]
pub struct StreamingDatabase {
    inner: wrapper::Database,
}

impl StreamingDatabase {
    /// Create a new database with the given patterns
    pub fn new(patterns: Vec<Pattern>) -> Result<Self, Error> {
        let inner = wrapper::Database::new(patterns, ScanMode::STREAM)?;
        Ok(Self { inner })
    }

    /// Create a new scanner from this database
    pub fn create_scanner(self: &Arc<Self>) -> Result<StreamingScanner, Error> {
        StreamingScanner::new(self.clone())
    }

    /// Get the size in bytes of the database
    pub fn size(&self) -> Result<usize, Error> {
        self.inner.size()
    }

    /// Get the size in bytes of a stream for this database database
    pub fn stream_size(&self) -> Result<usize, Error> {
        self.inner.stream_size()
    }

    /// Serializes the database using `hs_serialize_database`.
    pub fn serialize(&self) -> Result<SerializedDatabase, Error> {
        self.inner.serialize()
    }

    /// Deserializes a database using `hs_deserialize_database`.
    pub fn deserialize(sdb: SerializedDatabase) -> Result<Self, Error> {
        let db = wrapper::Database::deserialize(sdb)?;
        Ok(Self { inner: db })
    }
}

// -------------------------------------------------------------------------------------------------
// Stream
// -------------------------------------------------------------------------------------------------

/// A pattern matching state can be maintained across multiple blocks of target data
#[derive(Debug)]
pub struct Stream {
    inner: *mut hs::hs_stream_t,
}

impl Stream {
    fn new(database: &StreamingDatabase) -> Result<Self, Error> {
        let mut inner = MaybeUninit::zeroed();
        let flags = 0;
        unsafe {
            hs::hs_open_stream(database.inner.as_ptr(), flags, inner.as_mut_ptr())
                .ok()
                .map(|()| Self {
                    inner: inner.assume_init(),
                })
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Streaming Scanner
// -------------------------------------------------------------------------------------------------

/// A scanner that supports Vectorscan's streaming matching APIs
#[derive(Clone, Debug)]
pub struct StreamingScanner {
    scratch: wrapper::Scratch,
    db: Arc<StreamingDatabase>,
}

/// A scanner that supports Vectorscan’s stream-based matching APIs
#[derive(Debug)]
pub struct StreamScanner {
    scanner: Arc<StreamingScanner>,
    stream: Stream,
}

impl StreamingScanner {
    /// Create a new scanner with the given database
    pub fn new(db: Arc<StreamingDatabase>) -> Result<Self, Error> {
        Ok(Self {
            scratch: wrapper::Scratch::new(&db.inner)?,
            db,
        })
    }

    /// Open a new `Stream` object using `hs_open_stream`
    pub fn open_stream(self: &Arc<Self>) -> Result<StreamScanner, Error> {
        let stream = Stream::new(&self.db)?;
        Ok(StreamScanner {
            stream,
            scanner: self.clone(),
        })
    }
}

impl StreamScanner {
    /// Close the given `Stream` object using `hs_close_stream`.
    pub fn close<F>(self, on_match: F) -> Result<Scan, Error>
    where
        F: FnMut(u32, u64, u64, u32) -> Scan,
    {
        let mut context = Context { on_match };

        let res = unsafe {
            hs::hs_close_stream(
                self.stream.inner,
                self.scanner.scratch.as_ptr(),
                Some(on_match_trampoline::<F>),
                &mut context as *mut _ as *mut c_void,
            )
            .ok()
        };

        match res {
            Ok(_) => Ok(Scan::Continue),
            Err(err) => match err {
                Error::Hyperscan(HyperscanErrorCode::ScanTerminated) => Ok(Scan::Terminate),
                err => Err(err),
            },
        }
    }

    /// Scan the input using the given callback function
    ///
    /// The callback function takes 4 arguments and returns a `Scan` value.
    /// The 4 arguments:
    ///
    /// - id: u32     The ID of the expression that matched
    /// - from: u64   The offset of the start byte of the match; in practice, always 0
    /// - to: u64     The offset of the byte after the end byte of the match
    /// - flags: u32  Unused; "provided for future use"
    ///
    /// For more detail, see the Hyperscan documentation:
    ///
    /// - [`hs_scan_stream`](https://intel.github.io/hyperscan/dev-reference/api_files.html#c.hs_scan_stream)
    /// - [`match_event_handler`](https://intel.github.io/hyperscan/dev-reference/api_files.html#c.match_event_handler)
    pub fn scan<F>(&mut self, data: &[u8], on_match: F) -> Result<Scan, Error>
    where
        F: FnMut(u32, u64, u64, u32) -> Scan,
    {
        let mut context = Context { on_match };

        let res = unsafe {
            hs::hs_scan_stream(
                self.stream.inner,
                data.as_ptr() as *const _,
                data.len() as u32,
                0,
                self.scanner.scratch.as_ptr(),
                Some(on_match_trampoline::<F>),
                &mut context as *mut _ as *mut c_void,
            )
            .ok()
        };

        match res {
            Ok(_) => Ok(Scan::Continue),
            Err(err) => match err {
                Error::Hyperscan(HyperscanErrorCode::ScanTerminated) => Ok(Scan::Terminate),
                err => Err(err),
            },
        }
    }
}

// -------------------------------------------------------------------------------------------------
// User Context
// -------------------------------------------------------------------------------------------------

/// Bundles together Rust state to be passed to a C FFI Hyperscan matching API
///
/// This serves to wrap a Rust closure with a layer of indirection, so it can be referred to
/// through a `void *` pointer in C.
struct Context<F>
where
    F: FnMut(u32, u64, u64, u32) -> Scan,
{
    on_match: F,
}

unsafe extern "C" fn on_match_trampoline<F>(
    id: c_uint,
    from: c_ulonglong,
    to: c_ulonglong,
    flags: c_uint,
    ctx: *mut c_void,
) -> c_int
where
    F: FnMut(u32, u64, u64, u32) -> Scan,
{
    let context = unsafe {
        (ctx as *mut Context<F>)
            .as_mut()
            .expect("context object should be set")
    };
    match (context.on_match)(id, from, to, flags) {
        Scan::Continue => 0,
        Scan::Terminate => 1,
    }
}
