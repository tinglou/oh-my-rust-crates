#include "gzip_file.h"
#include "zlib.h"

#include <fstream>
#include <iostream>

#define CHUNK 16384

int gzip_file(const char* source_path, const char* dest_path) {
    int ret = 0;
    std::ifstream ifs (source_path, std::ios::binary);

    gzFile file = gzopen(dest_path, "wb9");
    if (file == NULL) {
        ret = ERFS_GZIP_DEST_NOT_FOUND;
        return ret;
    }
    unsigned char buf[CHUNK];
    int len;

    for(;;) {
        ifs.read(reinterpret_cast<char*>(buf), CHUNK);
        len = ifs.gcount();
        if (len == 0) {
            break;
        }
        if (gzwrite(file, buf, (unsigned)len) != len) {
            ret = ERFS_GZIP_COMPRESS_FAIL;
            break;
        }
    } 
    if (gzclose(file) != Z_OK) {
        ret = ERFS_GZIP_COMPRESS_FAIL;
        return ret;
    }
    return ret;
}



