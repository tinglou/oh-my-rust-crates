#define __ERFS_IMPL__
#include "resource_fs.h"

#define CHECK_NULL(V)       if(V == 0) {return ERFS_INVALID_INPUT;}

static inline const ErfsEntry* erfs_entry_at(const ErfsRoot fs, uint16_t index) {
    return (const ErfsEntry*)(fs->data + (uint32_t)index * sizeof(ErfsEntry));
}

static inline const ErfsEntry* erfs_dir_children_at(const ErfsRoot fs, const ErfsHandle dir) {
    return (const ErfsEntry*)(fs->data + dir->data_offset);
}

int erfs_read(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, const uint8_t **out, uint32_t *size) {
    ErfsHandle handle;
    int result = erfs_open(fs, path, path_len, &handle, size);
    if (result != 0) {
        return result;
    }
    if ((handle->flags & ERFS_DIRECTORY) != 0) {
        return ERFS_NOT_FILE;
    }
    *out = fs->data + handle->data_offset;
    return ERFS_OK;
}

static int strcmp_withlength(const int8_t *s1, int l1, const int8_t *s2, int l2) {
    int minlen = (l1 < l2) ? l1 : l2;
    for (int i = 0; i < minlen; i++, s1++, s2++) {
        if (*s1 < *s2) {
            return -1;
        } else if (*s1 > *s2) {
            return 1;
        }
    }
    if (l1 < l2) {
        return -1;
    } else if (l1 > l2) {
        return 1;
    }
    return 0;
}

static int erfs_binarysearch(const ErfsRoot fs, const ErfsHandle dir, const uint8_t *name, int len, ErfsHandle *out) {
    const ErfsEntry* A = erfs_dir_children_at(fs, dir);
    int L = 0;
    int R = (int)dir->data_size - 1;
    int m;

    const ErfsEntry* mentry;
    const uint8_t* mname;
    int mlen;
    int cmp;
    while (L <= R) {
        m = (L + R) / 2;
        mentry = A + m;
        mname = fs->data + mentry->name_offset;
        mlen = mentry->name_size;

        cmp = strcmp_withlength((const int8_t*)mname, mlen, (const int8_t*)name, len);
        if (cmp < 0) {
            L = m + 1;
        } else if (cmp > 0) {
            R = m - 1;
        } else {
            *out = mentry;
            return ERFS_OK;
        }
    }
    *out = 0;
    return ERFS_NOT_FOUND;
}

int erfs_open(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, ErfsHandle *out, uint32_t *size) {
    CHECK_NULL(fs);
    CHECK_NULL(path);
    CHECK_NULL(out);
    CHECK_NULL(size);
    const uint8_t *path_end = path + path_len;

    const uint8_t *pos = path;
    if (path_len > 0 && *pos == '/') {
        pos++;
    }

    const ErfsEntry* dir = erfs_entry_at(fs, 0);
    if (pos == path_end) {
        *out = dir;
        *size = dir->data_size;
        return ERFS_OK;
    }

    int result;
    const ErfsEntry* entry = 0;
    const uint8_t *start = pos;
    int len;
    while (pos != path_end) {
        while (*pos != '/' && pos != path_end) pos++;
        len = pos - start;

        result = erfs_binarysearch(fs, dir, start, len, &entry);
        if (result != ERFS_OK) {
            return result;
        }
        if (pos == path_end) {
            break;
        }
        if ((entry->flags & ERFS_DIRECTORY) == 0) {
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

int erfs_entryflags(const ErfsHandle handle, uint32_t *flags) {
    CHECK_NULL(handle);
    *flags = handle->flags;
    return ERFS_OK;
}

int erfs_entrysize(const ErfsHandle handle, uint32_t *size) {
    CHECK_NULL(handle);
    *size = handle->data_size;
    return ERFS_OK;
}

int erfs_entryname(const ErfsRoot fs, const ErfsHandle handle, const uint8_t **out, uint32_t *size) {
    CHECK_NULL(fs);
    CHECK_NULL(handle);
    CHECK_NULL(out);
    CHECK_NULL(size);
    *out = fs->data + handle->name_offset;
    *size = handle->name_size;
    return ERFS_OK;
}

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

int erfs_readdir(const ErfsRoot fs, const ErfsHandle handle, uint32_t index, ErfsHandle *out) {
    CHECK_NULL(fs);
    CHECK_NULL(handle);
    CHECK_NULL(out);
    if ((handle->flags & ERFS_DIRECTORY) == 0) {
        return ERFS_NOT_DIRECTORY;
    }
    if (index >= handle->data_size) {
        return ERFS_OUTOF_BOUND;
    }
    *out = erfs_dir_children_at(fs, handle) + index;
    return ERFS_OK;
}

static int erfs_travel_itr(const ErfsRoot fs, const ErfsEntry* handle, ErfsVisitFn func, void* ctx) {
    int result = 0;

    if ((handle->flags & ERFS_DIRECTORY) != 0) {
        result = (*func)(fs, handle, ERFS_TRAVEL_DIR_ENTER, ctx);
        if (result) {
            return result;
        }

        const ErfsEntry* children = erfs_dir_children_at(fs, handle);
        uint32_t count = handle->data_size;
        for (uint32_t i = 0; i < count; i++) {
            result = erfs_travel_itr(fs, children + i, func, ctx);
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

int erfs_travel(const ErfsRoot fs, ErfsVisitFn func, void* ctx) {
    CHECK_NULL(fs);
    CHECK_NULL(func);

    const ErfsEntry* dir = erfs_entry_at(fs, 0);
    CHECK_NULL(dir);

    return erfs_travel_itr(fs, dir, func, ctx);
}

ErfsRoot erfs_from_buffer(const void* buffer, uint32_t size) {
    if (!buffer || size < sizeof(ErfsFileSystem)) {
        return 0;
    }
    const ErfsFileSystem* fs = (const ErfsFileSystem*)buffer;
    if (fs->magic != ERFS_MAGIC) {
        return 0;
    }
    if (fs->total_size != size) {
        return 0;
    }
    return fs;
}
