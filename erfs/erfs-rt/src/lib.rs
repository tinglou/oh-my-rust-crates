//! # erfs-rt (Embedded Resource Filesystem - runtime libarary)
//!
//! `erfs-rt` access the generated fs.


#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
mod erfs_binding;

use std::slice;

/// handle of a ERFS instance, returned by the generated codes.
pub use erfs_binding::ErfsRoot;

/// handle of a directory entry
pub use erfs_binding::ErfsHandle;

/// get content of specified file. 
pub fn read(fs: ErfsRoot, path: &str) -> Result<&'static [u8], i32> {
    let mut buf = 0 as *const u8;
    let mut size :u32 = 0;
    let pbuf = &mut buf as *mut *const u8;
    let psize = &mut size as *mut u32;

    let ret :i32;
    unsafe { 
        ret = erfs_binding::erfs_read(fs, path.as_ptr(), path.len() as u32, pbuf, psize);
    }
    if ret == 0 {
        unsafe {
            Ok(slice::from_raw_parts(buf, size as usize))
        }
    } else {
        Err(ret)
    }  
}

/// get directory entry handle from pathname.
pub fn open(fs: ErfsRoot, path: &str) -> Result<(ErfsHandle, u32), i32> {
    let mut handle: ErfsHandle = 0 as ErfsHandle;
    let mut size :u32 = 0;
    let phandle = &mut handle as *mut ErfsHandle;
    let psize = &mut size as *mut u32;

    let ret :i32;
    unsafe { 
        ret = erfs_binding::erfs_open(fs, path.as_ptr(), path.len() as u32, phandle, psize);
    }
    if ret == 0 {
        Ok((handle, size as u32))
    } else {
        Err(ret)
    }  
}

/// get flags of the specified directory entry.
pub fn entry_flags(entry: ErfsHandle) -> Result<u32, i32> {
    let mut flags :u32 = 0;
    let pflags = &mut flags as *mut u32;
    let ret:i32;
    unsafe { 
        ret = erfs_binding::erfs_entryflags(entry, pflags);
    }
    if ret == 0 {
        Ok(flags)
    } else {
        Err(ret as i32)
    }
}

/// get size of the specified directory entry.
pub fn entry_size(entry: ErfsHandle) -> Result<u32, i32> {
    let mut size :u32 = 0;
    let psize = &mut size as *mut u32;
    let ret:i32;
    unsafe { 
        ret = erfs_binding::erfs_entryflags(entry, psize);
    }
    if ret == 0 {
        Ok(size)
    } else {
        Err(ret as i32)
    }
}

/// get file name of a directory entry
pub fn entry_name(fs: ErfsRoot, entry: ErfsHandle) -> Result<&'static [u8], i32> {
    let mut buf = 0 as *const u8;
    let mut size :u32 = 0;
    let pbuf = &mut buf as *mut *const u8;
    let psize = &mut size as *mut u32;

    let ret :i32;
    unsafe { 
        ret = erfs_binding::erfs_entryname(fs, entry, pbuf, psize);
    }
    if ret == 0 {
        unsafe {
            Ok(slice::from_raw_parts(buf, size as usize))
        }
    } else {
        Err(ret)
    } 
}

/// get content of a directory entry
pub fn read_file(fs: ErfsRoot, entry: ErfsHandle) -> Result<&'static [u8], i32> {
    let mut buf = 0 as *const u8;
    let mut size :u32 = 0;
    let pbuf = &mut buf as *mut *const u8;
    let psize = &mut size as *mut u32;

    let ret :i32;
    unsafe { 
        ret = erfs_binding::erfs_readfile(fs, entry, pbuf, psize);
    }
    if ret == 0 {
        unsafe {
            Ok(slice::from_raw_parts(buf, size as usize))
        }
    } else {
        Err(ret)
    } 
}

/// read a directory
pub fn read_dir(fs: ErfsRoot, dir: ErfsHandle, index: u32) -> Result<ErfsHandle, i32> {
    let mut handle: ErfsHandle = 0 as ErfsHandle;
    let phandle = &mut handle as *mut ErfsHandle;

    let ret :i32;
    unsafe { 
        ret = erfs_binding::erfs_readdir(fs, dir, index, phandle);
    }
    if ret == 0 {
        Ok(handle)
    } else {
        Err(ret)
    } 
}

/*
use erfs_binding::ErfsVisitFn;
pub fn erfs_travel(fs: ErfsRoot, func: ErfsVisitFn, ctx: *mut ::std::os::raw::c_void) -> i32 {
    unsafe { 
        erfs_binding::erfs_travel(fs, func, ctx) 
    }
}
*/