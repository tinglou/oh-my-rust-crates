#define __ERFS_IMPL__
#include "resource_fs.h"
#include "erfs_test.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int test_count = 0;
static int pass_count = 0;

#define ASSERT_EQ(a, b, msg) do { \
    test_count++; \
    if ((a) == (b)) { pass_count++; } \
    else { printf("FAIL: %s (expected %d, got %d)\n", msg, (int)(b), (int)(a)); } \
} while(0)

#define ASSERT_STR_EQ(a, alen, b, msg) do { \
    test_count++; \
    if ((alen) == (int)strlen(b) && memcmp(a, b, alen) == 0) { pass_count++; } \
    else { printf("FAIL: %s\n", msg); } \
} while(0)

static int visit_fn(const ErfsRoot fs, const ErfsHandle entry, enum ErfsTravelType type, void* ctx) {
    int* count = (int*)ctx;
    (*count)++;
    return 0;
}

static void test_fs(const ErfsRoot fs, const char* label) {
    printf("\n=== Testing: %s ===\n", label);

    ErfsHandle handle;
    uint32_t size;
    int rc;

    rc = erfs_open(fs, (const uint8_t*)"/", 1, &handle, &size);
    ASSERT_EQ(rc, ERFS_OK, "open root");

    const uint8_t* name;
    uint32_t name_size;
    rc = erfs_entryname(fs, handle, &name, &name_size);
    ASSERT_EQ(rc, ERFS_OK, "root entryname");
    ASSERT_STR_EQ(name, name_size, "/", "root name is /");

    rc = erfs_open(fs, (const uint8_t*)"/gen", 4, &handle, &size);
    ASSERT_EQ(rc, ERFS_OK, "open /gen");

    rc = erfs_entryflags(handle, &size);
    ASSERT_EQ(rc, ERFS_OK, "gen entryflags");
    ASSERT_EQ(size & ERFS_DIRECTORY, ERFS_DIRECTORY, "/gen is directory");

    rc = erfs_open(fs, (const uint8_t*)"/gen/main.cpp", 13, &handle, &size);
    ASSERT_EQ(rc, ERFS_OK, "open /gen/main.cpp");

    rc = erfs_entryflags(handle, &size);
    ASSERT_EQ(rc, ERFS_OK, "main.cpp entryflags");
    ASSERT_EQ(size & ERFS_DIRECTORY, 0, "/gen/main.cpp is file");

    const uint8_t* content;
    uint32_t content_size;
    rc = erfs_read(fs, (const uint8_t*)"/gen/main.cpp", 13, &content, &content_size);
    ASSERT_EQ(rc, ERFS_OK, "read /gen/main.cpp");
    ASSERT_EQ(content_size > 0, 1, "main.cpp has content");

    rc = erfs_read(fs, (const uint8_t*)"/gen/erfs_generator.h", 21, &content, &content_size);
    ASSERT_EQ(rc, ERFS_OK, "read /gen/erfs_generator.h");

    rc = erfs_read(fs, (const uint8_t*)"/nonexistent", 12, &content, &content_size);
    ASSERT_EQ(rc, ERFS_NOT_FOUND, "read nonexistent returns NOT_FOUND");

    int travel_count = 0;
    rc = erfs_travel(fs, visit_fn, &travel_count);
    ASSERT_EQ(rc, ERFS_OK, "travel");
    ASSERT_EQ(travel_count > 0, 1, "travel visited entries");
    printf("  traveled %d entries\n", travel_count);
}

int main(int argc, char** argv) {
    ErfsRoot embedded = erfs_test();
    test_fs(embedded, "Embedded (from generated .c)");

    const char* img_path = "erfs_test.img";
    if (argc > 1) {
        img_path = argv[1];
    }

    FILE* f = fopen(img_path, "rb");
    if (f) {
        fseek(f, 0, SEEK_END);
        long fsize = ftell(f);
        fseek(f, 0, SEEK_SET);
        void* buf = malloc(fsize);
        fread(buf, 1, fsize, f);
        fclose(f);

        ErfsRoot from_file = erfs_from_buffer(buf, (uint32_t)fsize);
        if (from_file) {
            test_fs(from_file, "From .img file (erfs_from_buffer)");
        } else {
            printf("FAIL: erfs_from_buffer returned NULL\n");
        }
        free(buf);
    } else {
        printf("SKIP: %s not found (run with --image to generate)\n", img_path);
    }

    printf("\n%d/%d tests passed\n", pass_count, test_count);
    return (pass_count == test_count) ? 0 : 1;
}
