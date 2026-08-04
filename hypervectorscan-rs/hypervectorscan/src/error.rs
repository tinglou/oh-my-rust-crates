use hypervectorscan_sys as ffi;
use thiserror::Error;

/// Hyperscan Error Codes
#[derive(Debug, Error)]
pub enum Error {
    #[error("Pattern expression contains NUL byte")]
    Nul(#[from] std::ffi::NulError),

    #[error("Error originating from Hyperscan API")]
    Hyperscan(HyperscanErrorCode),

    #[error("Pattern comilation failed, {0} at {1}")]
    HyperscanCompile(String, i32),
}

/// error code definition
#[repr(i32)]
#[derive(Debug, PartialEq, Eq)]
pub enum HyperscanErrorCode {
    /// A parameter passed to this function was invalid.
    ///
    /// This error is only returned in cases where the function can
    /// detect an invalid parameter it cannot be relied upon to detect
    /// (for example) pointers to freed memory or other invalid data.
    Invalid = ffi::HS_INVALID,

    /// A memory allocation failed.
    Nomem = ffi::HS_NOMEM,

    /// The engine was terminated by callback.
    ///
    ///  This return value indicates that the target buffer was
    ///  partially scanned, but that the callback function requested
    ///  that scanning cease after a match was located.
    ScanTerminated = ffi::HS_SCAN_TERMINATED,

    /// The pattern compiler failed, and the hs_compile_error_t should
    /// be inspected for more detail.
    CompilerError = ffi::HS_COMPILER_ERROR,

    /// The given database was built for a different version of Hyperscan.
    DbVersionError = ffi::HS_DB_VERSION_ERROR,

    /// The given database was built for a different platform (i.e., CPU type).
    DbPlatformError = ffi::HS_DB_PLATFORM_ERROR,

    /// The given database was built for a different mode of
    /// operation. This error is returned when streaming calls are
    /// used with a block or vectored database and vice versa.
    DbModeError = ffi::HS_DB_MODE_ERROR,

    /// A parameter passed to this function was not correctly aligned.
    BadAlign = ffi::HS_BAD_ALIGN,

    /// The memory allocator (either malloc() or the allocator set
    /// with hs_set_allocator()) did not correctly return memory
    /// suitably aligned for the largest representable data type on
    /// this platform.
    BadAlloc = ffi::HS_BAD_ALLOC,

    /// The scratch region was already in use.
    ///
    /// s error is returned when Hyperscan is able to detect that the
    /// scratch region given is already in use by another Hyperscan
    /// API call.
    ///
    /// A separate scratch region, allocated with hs_alloc_scratch()
    /// or hs_clone_scratch(), is required for every concurrent caller
    /// of the Hyperscan API.
    ///
    /// For example, this error might be returned when hs_scan() has
    /// been called inside a callback delivered by a
    /// currently-executing hs_scan() call using the same scratch
    /// region.
    ///
    /// Note: Not all concurrent uses of scratch regions may be
    /// detected. This error is intended as a best-effort debugging
    /// tool, not a guarantee.
    ScratchInUse = ffi::HS_SCRATCH_IN_USE,

    /// Unsupported CPU architecture.
    ///
    /// This error is returned when Hyperscan is able to detect that
    /// the current system does not support the required instruction
    /// set.
    ///
    /// At a minimum, Hyperscan requires Supplemental Streaming SIMD
    /// Extensions 3 (SSSE3).
    ArchError = ffi::HS_ARCH_ERROR,

    /// Provided buffer was too small.
    ///
    /// This error indicates that there was insufficient space in the
    /// buffer. The call should be repeated with a larger provided
    /// buffer.
    ///
    /// Note: in this situation, it is normal for the amount of space
    /// required to be returned in the same manner as the used space
    /// would have been returned if the call was successful.
    InsufficientSpace = ffi::HS_INSUFFICIENT_SPACE,

    /// Unexpected internal error.
    ///
    /// This error indicates that there was unexpected matching
    /// behaviors. This could be related to invalid usage of stream
    /// and scratch space or invalid memory operations by users.
    UnknownError = ffi::HS_UNKNOWN_ERROR,

    UnknownErrorCode = -100,
}

impl From<ffi::hs_error_t> for HyperscanErrorCode {
    fn from(err: ffi::hs_error_t) -> Self {
        // 若错误码在 Hyperscan C API 定义的范围内（-13..=-1），直接安全地转换为枚举。
        // 否则返回 UnknownErrorCode。
        if (-13..=-1).contains(&err) {
            // SAFETY: repr(i32) 且所有 -1..=-13 的值都已在 HyperscanErrorCode 枚举中定义
            unsafe { std::mem::transmute::<i32, HyperscanErrorCode>(err) }
        } else {
            Self::UnknownErrorCode
        }
    }
}

impl From<ffi::hs_error_t> for Error {
    fn from(err: ffi::hs_error_t) -> Self {
        Error::Hyperscan(err.into())
    }
}
pub trait AsResult: Sized {
    fn ok(self) -> Result<(), Error>;
}

impl AsResult for ffi::hs_error_t {
    fn ok(self) -> Result<(), Error> {
        if self == ffi::HS_SUCCESS as ffi::hs_error_t {
            Ok(())
        } else {
            Err(self.into())
        }
    }
}
