
use std::ffi::CStr;

/**
 enum RfsGzipStatusCode {
    ERFS_GZIP_OK                 = 0,
    ERFS_GZIP_SRC_NOT_FOUND      = -1,
    ERFS_GZIP_DEST_NOT_FOUND     = -2,
    ERFS_GZIP_COMPRESS_FAIL      = -3,
    ERFS_GZIP_COMPRESS_RATIO     = -4,
*/

///implement the C interface and called by erfs_generator.cpp
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn gzip_file(source_path: *const ::std::os::raw::c_char, 
    dest_path: *const ::std::os::raw::c_char) -> ::std::os::raw::c_int {

    unsafe {
        let source = match CStr::from_ptr(source_path).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let dest = match CStr::from_ptr(dest_path).to_str() {
            Ok(s) => s,
            Err(_) => return -2,
        };
        gzip_file_rs(&source, &dest)
    }
}

fn gzip_file_rs(source: &str, dest: &str) -> i32 {
    use std::fs;

    use std::io::Write;

    use deflate::Compression;
    use deflate::write::ZlibEncoder;
    
    let data = fs::read(source);
    let data = match data {
        Ok(d) => d,
        Err(_) => return -1,
    };

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::Best);
    match encoder.write_all(data.as_slice()) {
        Ok(_) => (),
        Err(_) => return -3,
    }
    
    let compressed_data = match encoder.finish() {
        Ok(data) => data,
        Err(_) => return -3,
    };

    match fs::write(dest, compressed_data) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}
