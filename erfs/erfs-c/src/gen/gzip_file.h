#pragma once

#if defined(__cplusplus)
extern "C" {
#endif


///
///
enum RfsGzipStatusCode {
    ERFS_GZIP_OK                 = 0,
    ERFS_GZIP_SRC_NOT_FOUND      = -1,
    ERFS_GZIP_DEST_NOT_FOUND     = -2,
    ERFS_GZIP_COMPRESS_FAIL      = -3,
    ERFS_GZIP_COMPRESS_RATIO     = -4,
};

///
/// @return 0:success, -1:failed to open file to read; -2: failed to open file to write; -3: compress fail; -4: needn't compress
///
int gzip_file(const char* source_path, const char* dest_path);


#if defined(__cplusplus)
}
#endif