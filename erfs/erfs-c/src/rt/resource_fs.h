#pragma once

#if defined(__ERFS_IMPL__)
#include <stdint.h>
#if defined(__cplusplus)
extern "C" {
#endif

#pragma pack(1)

/// a directory or file
typedef struct  {
    uint32_t name_offset;
    uint32_t name_size;
    uint32_t data_offset;
    uint32_t data_size;
    uint32_t flags;
} ErfsEntry;

typedef const ErfsEntry* ErfsHandle;

/// the whole resource filesystem
typedef struct {
    // all entries including directories and files
    uint32_t entry_count;
    ErfsEntry *entries;

    // buffer to hold all names and contents
    uint32_t data_size;
    uint8_t  *data;
} ErfsFileSystem;

typedef const ErfsFileSystem * ErfsRoot;
#pragma pack()

#if defined(__cplusplus)
}
#endif

#else // defined(ERFS_IMPL)
typedef unsigned int uint32_t;
typedef unsigned char uint8_t;
// https://stackoverflow.com/questions/4079243/how-can-i-use-sizeof-in-a-preprocessor-macro
//https://stackoverflow.com/questions/1597007/creating-c-macro-with-and-line-token-concatenation-with-positioning-macr
#define BUILD_BUG_ON(condition) typedef char p__LINE__ [ (condition) ? -1 : 1];
BUILD_BUG_ON( sizeof(uint32_t) != 4 );

typedef const void* ErfsRoot;
typedef const void* ErfsHandle;
#endif // defined(ERFS_IMPL)

#if defined(__cplusplus)
extern "C" {
#endif

///
/// ERFSEntry flags
///
enum ErfsEntryFlags {
    ERFS_DIRECTORY       = 1,
    ERFS_GZIPPED         = 2, 
};

///
/// return code of access api
///
enum ErfsStatusCode {
    ERFS_OK                      = 0,
    ERFS_INVALID_INPUT           = -1,
    ERFS_NOT_FOUND               = -2,
    ERFS_NOT_FILE                = -3,
    ERFS_NOT_DIRECTORY           = -4,
    ERFS_OUTOF_BOUND             = -5,
};

/// read a regular file
///@param fs the file system
///@param path the file name to read
///@param path_len length of path
///@param out pointer to the content
///@param size file size
///@return 0 for success; other for notfound
int erfs_read(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, const uint8_t **out, uint32_t *size);

/// open a FS entry
///@param fs the file system
///@param path the file name to read
///@param path_len length of path
///@param out handle
///@param size file size or entries in the directory
///@return 0 for success; other for notfound
int erfs_open(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, ErfsHandle *out, uint32_t *size);

/// get flags of an entry (directry or file)
///@param entry entry (directry or file)
///@param flags [out] flags
///@return flags
int erfs_entryflags(const ErfsHandle entry, uint32_t *flags);

/// get size of an entry (directry or file)
///@param entry entry (directry or file)
///@param flags [out] size
///@return size of file or dirctory
int erfs_entrysize(const ErfsHandle entry, uint32_t *size);

/// get name of an entry (directry or file)
///@param fs the file system
///@param entry entry (directry or file)
///@param out pointer to the content
///@param size file size
///@return 0 for success; other for notfound
int erfs_entryname(const ErfsRoot fs, const ErfsHandle entry, const uint8_t **out, uint32_t *size);

/// read a regular file
///@param fs the file system
///@param entry the file name
///@param out pointer to the content
///@param size file size
///@return 0 for success; other for notfound
int erfs_readfile(const ErfsRoot fs, const ErfsHandle entry, const uint8_t **out, uint32_t *size);

/// get flags of an entry (directry or file)
///@param fs the file system
///@param dir directory to read
///@param index entry index
///@param out out entry
///@return 0 for success; other for notfound
int erfs_readdir(const ErfsRoot fs, const ErfsHandle dir, uint32_t index, ErfsHandle *out);


enum ErfsTravelType {
    ERFS_TRAVEL_DIR_ENTER,
    ERFS_TRAVEL_DIR_LEAVE,
    ERFS_TRAVEL_FILE,
};

typedef int (*ErfsVisitFn) (const ErfsRoot fs, const ErfsHandle entry, enum ErfsTravelType type, void* ctx);

/// travel the resource filesystem
///@param fs the file system
///@param func callback function
///@param ctx context
///@return 0 for success; other for notfound
int erfs_travel(const ErfsRoot fs, ErfsVisitFn func, void* ctx);

#if defined(__cplusplus)
}
#endif
