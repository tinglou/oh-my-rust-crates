#pragma once

#if defined(__cplusplus)
extern "C" {
#endif

enum ErfsGenOption {
    ERFS_GEN_IMAGE            = 8,
};

enum ErfsGenStatusCode {
    ERFS_GEN_OK                  = 0,
    ERFS_INVALID_INPUT           = -1,
    ERFS_NOT_FOUND               = -2,
    ERFS_NOT_FILE                = -3,
    ERFS_NOT_DIRECTORY           = -4,
    ERFS_OUTOF_BOUND             = -5,

    ERFS_SOURCE_TOO_LARGE        = -100,
    ERFS_TARGET_NOT_EXIST        = -101,
    ERFS_INVALID_ID              = -102,
    ERFS_INVALID_OPTION          = -103,
};

int erfs_generate(const char *path, const char *id, int options, const char *target_dir);

#if defined(__cplusplus)
}
#endif
