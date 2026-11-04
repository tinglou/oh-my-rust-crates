#pragma once

#if defined(__ERFS_IMPL__)
#include <stdint.h>
#if defined(__cplusplus)
extern "C" {
#endif

#define ERFS_MAGIC 0x45524653

#pragma pack(1)

typedef struct {
    uint32_t data_offset;
    uint32_t data_size;
    uint32_t name_offset;
    uint16_t name_size;
    uint16_t flags;
} ErfsEntry;

typedef const ErfsEntry* ErfsHandle;

typedef struct {
    uint32_t magic;
    uint32_t total_size;
    uint16_t entry_count;
    uint16_t _reserved;
    uint8_t  data[0];
} ErfsFileSystem;

typedef const ErfsFileSystem* ErfsRoot;
#pragma pack()

#if defined(__cplusplus)
}
#endif

#else // defined(__ERFS_IMPL__)
typedef unsigned int uint32_t;
typedef unsigned short uint16_t;
typedef unsigned char uint8_t;
#define BUILD_BUG_ON(condition) typedef char p__LINE__ [ (condition) ? -1 : 1];
BUILD_BUG_ON( sizeof(uint32_t) != 4 );

typedef const void* ErfsRoot;
typedef const void* ErfsHandle;
#endif // defined(__ERFS_IMPL__)

#if defined(__cplusplus)
extern "C" {
#endif

enum ErfsEntryFlags {
    ERFS_DIRECTORY       = 1,
    ERFS_GZIPPED         = 2,
};

enum ErfsStatusCode {
    ERFS_OK                      = 0,
    ERFS_INVALID_INPUT           = -1,
    ERFS_NOT_FOUND               = -2,
    ERFS_NOT_FILE                = -3,
    ERFS_NOT_DIRECTORY           = -4,
    ERFS_OUTOF_BOUND             = -5,
    ERFS_INVALID_MAGIC           = -6,
    ERFS_INVALID_SIZE            = -7,
};

int erfs_read(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, const uint8_t **out, uint32_t *size);

int erfs_open(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, ErfsHandle *out, uint32_t *size);

int erfs_entryflags(const ErfsHandle entry, uint32_t *flags);

int erfs_entrysize(const ErfsHandle entry, uint32_t *size);

int erfs_entryname(const ErfsRoot fs, const ErfsHandle entry, const uint8_t **out, uint32_t *size);

int erfs_readfile(const ErfsRoot fs, const ErfsHandle entry, const uint8_t **out, uint32_t *size);

int erfs_readdir(const ErfsRoot fs, const ErfsHandle dir, uint32_t index, ErfsHandle *out);

enum ErfsTravelType {
    ERFS_TRAVEL_DIR_ENTER,
    ERFS_TRAVEL_DIR_LEAVE,
    ERFS_TRAVEL_FILE,
};

typedef int (*ErfsVisitFn) (const ErfsRoot fs, const ErfsHandle entry, enum ErfsTravelType type, void* ctx);

int erfs_travel(const ErfsRoot fs, ErfsVisitFn func, void* ctx);

ErfsRoot erfs_from_buffer(const void* buffer, uint32_t size);

#if defined(__cplusplus)
}
#endif
