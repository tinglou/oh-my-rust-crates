#define __ERFS_IMPL__
#include "resource_fs.h"

#define CHECK_NULL(V)       if(V == 0) {return ERFS_INVALID_INPUT;}

/// read a regular file
///@param fs the file system
///@param path the file name to read
///@param out pointer to the content
///@param size file size
///@return ERFS_OK for success; other for notfound
int erfs_read(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, const uint8_t **out, uint32_t *size) {
    ErfsHandle handle;
    int result = erfs_open (fs, path, path_len, &handle, size);
    if (result != 0) {
        return result;
    }
    if ((handle->flags & ERFS_DIRECTORY) != 0) {
        return ERFS_NOT_FILE;
    }
    *out = fs->data + handle->data_offset;
    // *size = handle->data_size;
    return ERFS_OK;
}


static int strcmp_withlength (const int8_t *s1, int l1, const int8_t *s2, int l2) {
    int minlen = (l1 < l2)? l1 : l2;
    for(int i = 0; i < minlen; i++, s1++, s2++){
        if(*s1 < *s2) {
            return -1;
        } else if(*s1 > *s2){
            return 1;
        }
    }
    if(l1 < l2) {
        return -1;
    } else if (l1 > l2) {
        return 1;
    }
    return 0;
}

static int erfs_binarysearch(const ErfsRoot fs, const ErfsHandle handle, const uint8_t *name, int len, ErfsHandle *out) {
    ErfsHandle A = fs->entries + handle->data_offset;
    int L = 0;
    int R = (handle->data_size - 1);
    int m;

    ErfsHandle mentry;
    uint8_t *mname;
    int mlen;
    int cmp;
    while (L <= R) {
        m = (L + R) / 2;
        mentry = A + m;
        mname = fs->data + mentry->name_offset;
        mlen = mentry->name_size;

        cmp = strcmp_withlength((const int8_t *)mname, mlen, (const int8_t *)name, len);
        if (cmp < 0) {
            L = m + 1;
        } else if (cmp > 0) {
            R = m - 1;
        } else {
            *out = mentry;
            return ERFS_OK;
        }
    }
    out = 0;
    return ERFS_NOT_FOUND;
}


/// open a FS entry
/// don't support "/../" or "/./"
///@param fs the file system
///@param path the file name to read
///@param out handle
///@param size file size or entries in the directory
///@return ERFS_OK for success; other for notfound
int erfs_open(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, ErfsHandle *out, uint32_t *size) {
    CHECK_NULL(fs);
    CHECK_NULL(path);
    CHECK_NULL(out);
    CHECK_NULL(size);
    const uint8_t *path_end = path + path_len;

    // if first character is '/', ignore
    const uint8_t *pos = path;
    if (path_len > 0 && *pos == '/') {
        pos++;
    }

    // open root dir
    ErfsHandle dir = fs->entries;
    if(pos == path_end){
        *out = dir;
        *size = dir->data_size;
        return ERFS_OK;
    }

    int result;
    ErfsHandle entry = 0;
    const uint8_t *start = pos;
    int len;
    while (pos != path_end) {
        while(*pos != '/' && pos != path_end) pos++;
        len = pos - start;

        result = erfs_binarysearch(fs, dir, start, len, &entry);
        if (result != ERFS_OK) {
            return result;
        }
        if (pos == path_end) {
            // path end
            break;
        }
        if ((entry->flags & ERFS_DIRECTORY) == 0) {
            // path isn't end but reach a regular file
            return ERFS_NOT_FOUND;
        }
        dir = entry;
        pos++;
        start = pos;
    }

    *out = entry;
    *size = entry->data_size;
    return ERFS_OK;
}

/// get flags of an entry (directry or file)
///@param entry entry (directry or file)
///@return ERFS_OK for success
int erfs_entryflags(const ErfsHandle handle, uint32_t *flags) {
    CHECK_NULL(handle);
    *flags = handle->flags;
    return ERFS_OK;
}

/// get flags of an entry (directry or file)
///@param handle entry (directry or file)
///@param flags [out] size
///@return size of file or dirctory
int erfs_entrysize(const ErfsHandle handle, uint32_t *size) {
    CHECK_NULL(handle);
    *size = handle->data_size;
    return ERFS_OK;    
}

/// get name of an entry (directry or file)
///@param fs the file system
///@param handle entry (directry or file)
///@param out pointer to the content
///@param size file size
///@return ERFS_OK for success; other for notfound
int erfs_entryname(const ErfsRoot fs, const ErfsHandle handle, const uint8_t **out, uint32_t *size) {
    CHECK_NULL(fs);
    CHECK_NULL(handle);
    CHECK_NULL(out);
    CHECK_NULL(size);
    *out = fs->data + handle->name_offset;
    *size = handle->name_size;
    return ERFS_OK;
}

/// read a regular file
///@param fs the file system
///@param handle the file name
///@param out pointer to the content
///@param size file size
///@return ERFS_OK for success; other for notfound
int erfs_readfile(const ErfsRoot fs, const ErfsHandle handle, const uint8_t **out, uint32_t *size) {
    CHECK_NULL(fs);
    CHECK_NULL(handle);
    CHECK_NULL(out);
    CHECK_NULL(size);
    if ((handle->flags & ERFS_DIRECTORY) != 0) {
        return ERFS_NOT_FILE;
    }
    *out = fs->data + handle->data_offset;
    *size = handle->data_size;
    return ERFS_OK;
}

/// get flags of an entry (directry or file)
///@param fs the file system
///@param handle directory to read
///@param index entry index
///@param out out entry
///@return ERFS_OK for success; other for notfound
int erfs_readdir(const ErfsRoot fs, const ErfsHandle handle, uint32_t index, ErfsHandle *out){
    CHECK_NULL(fs);
    CHECK_NULL(handle);
    CHECK_NULL(out);
    if ((handle->flags & ERFS_DIRECTORY) == 0) {
        return ERFS_NOT_DIRECTORY;
    }
    if (index >= handle->data_size) {
        return ERFS_OUTOF_BOUND;
    }
    *out = fs->entries + handle->data_offset;
    return ERFS_OK;
}


static int erfs_travel_itr(const ErfsRoot fs, ErfsHandle handle, ErfsVisitFn func, void* ctx) {
    int result = 0;

    if ((handle->flags & ERFS_DIRECTORY) != 0) {
        result = (*func)(fs, handle, ERFS_TRAVEL_DIR_ENTER, ctx);
        if (result) {
            return result;
        }

        // directory entries
        int size = handle->data_size;
        for (int i = 0; i < size; i++) {
            result = erfs_travel_itr(fs, fs->entries + (handle->data_offset + i), func, ctx);
            if (result) {
                return result;
            }
        }

        result = (*func)(fs, handle, ERFS_TRAVEL_DIR_LEAVE, ctx);
        if (result) {
            return result;
        }
    } else {
        result = (*func)(fs, handle, ERFS_TRAVEL_FILE, ctx);
        if (result) {
            return result;
        }
    }
    return result;
}

/// travel the resource filesystem
///@param fs the file system
///@param func callback function
///@param ctx context
///@return ERFS_OK for success; other for notfound
int erfs_travel(const ErfsRoot fs, ErfsVisitFn func, void* ctx) {
    CHECK_NULL(fs);
    CHECK_NULL(func);

    ErfsHandle dir = fs->entries;
    CHECK_NULL(dir);

    return erfs_travel_itr(fs, dir, func, ctx);
}
