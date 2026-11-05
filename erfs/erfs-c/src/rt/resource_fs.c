/**
 * @file resource_fs.c
 * @brief ERFS 运行时库实现
 */
#define __ERFS_IMPL__
#include "resource_fs.h"

/** @brief 空指针检查宏 */
#define CHECK_NULL(V)       if(V == 0) {return ERFS_INVALID_INPUT;}

/**
 * @brief 根据索引获取条目指针
 *
 * @param fs    文件系统根句柄
 * @param index 条目索引
 * @return 条目指针
 */
static inline const ErfsEntry* erfs_entry_at(const ErfsRoot fs, uint16_t index) {
    return (const ErfsEntry*)(fs->data + (uint32_t)index * sizeof(ErfsEntry));
}

/**
 * @brief 获取目录的子条目指针
 *
 * @param fs  文件系统根句柄
 * @param dir 目录条目句柄
 * @return 首个子条目指针
 */
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

/**
 * @brief 带长度限制的字符串比较
 *
 * @param s1  第一个字符串
 * @param l1  第一个字符串长度
 * @param s2  第二个字符串
 * @param l2  第二个字符串长度
 * @return <0: s1<s2, 0: 相等, >0: s1>s2
 */
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

/**
 * @brief 在目录中二分查找指定名称的条目
 *
 * 目录内条目按名称排序，使用二分查找提高效率。
 *
 * @param fs   文件系统根句柄
 * @param dir  目录条目句柄
 * @param name 要查找的名称
 * @param len  名称长度
 * @param out  [out] 找到的条目句柄
 * @return @ref ERFS_OK 成功，@ref ERFS_NOT_FOUND 未找到
 */
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

/**
 * @brief 递归遍历文件系统 (内部实现)
 *
 * @param fs     文件系统根句柄
 * @param handle 当前条目
 * @param func   回调函数
 * @param ctx    用户上下文
 * @return 0 继续，非 0 中止
 */
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
#if defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_BIG_ENDIAN__
    if (fs->magic != ERFS_MAGIC_BE) {
#else
    if (fs->magic != ERFS_MAGIC_LE) {
#endif
        return 0;
    }
    if (fs->total_size != size) {
        return 0;
    }
    return fs;
}
