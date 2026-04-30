#include "gtest/gtest.h"
#include "resource_fs.h"

#include "erfs_rfsrc.h"


namespace {
const ErfsRoot fs = erfs_gen_rfsrc();


extern "C" int list_callback (const ErfsRoot fs, const ErfsHandle entry, enum ErfsTravelType type, void* ctx) {
    int* indent = reinterpret_cast<int*>(ctx);
    int backup_indent = *indent;
    const uint8_t* name;
    uint32_t name_len;
    erfs_entryname(fs, entry, &name, &name_len);

    if (type == ERFS_TRAVEL_DIR_ENTER) {
        // enter directory
        (*indent)++;

        for (int i = 0; i < backup_indent; i++) {
            std::cout << "  ";
        }
        std::cout << std::string((char*)name, (int)name_len)  << " [DIR]" << std::endl;
    } else if (type == ERFS_TRAVEL_DIR_LEAVE) {
        // leave directory
        (*indent)--;
    } else {
        // file
        for (int i = 0; i < backup_indent; i++) {
            std::cout << "  ";
        }
        uint32_t flags;
        erfs_entryflags(entry, &flags);
        std::cout << std::string((char*)name, (int)name_len) << ((flags != 0) ? " [GZIPPED]" : "") << std::endl;
    }

    return 0;
}

TEST(RFS, travel) {
    int indent = 0;
    int result = erfs_travel(fs, list_callback, &indent);
    EXPECT_EQ(result, 0);
}

TEST(RFS, read_ok) {
    const uint8_t * buff;
    uint32_t size;
    int result = erfs_read(fs, (const uint8_t *)"/src/resource_fs.h", strlen("/src/resource_fs.h"), &buff, &size);
    EXPECT_EQ(result, ERFS_OK);
}

TEST(RFS, read_fail) {
    const uint8_t * buff;
    uint32_t size;
    int result;
    
    result = erfs_read(fs, (const uint8_t *)"/hello.h", strlen("/hello.h"), &buff, &size);
    EXPECT_EQ(result, ERFS_NOT_FOUND);

    result = erfs_read(fs, (const uint8_t *)"/", strlen("/"), &buff, &size);
    EXPECT_EQ(result, ERFS_NOT_FILE);

    result = erfs_read(NULL, (const uint8_t *)"/", strlen("/"), &buff, &size);
    EXPECT_EQ(result, ERFS_INVALID_INPUT);

    result = erfs_read(fs, (const uint8_t *)NULL, 0, &buff, &size);
    EXPECT_EQ(result, ERFS_INVALID_INPUT);

    // treat "" as "/"
    result = erfs_read(fs, (const uint8_t *)"", 0, &buff, &size);
    EXPECT_EQ(result, ERFS_NOT_FILE);
}


TEST(RFS, read_open_dir) {
    const uint8_t * buff;
    uint32_t size;
    ErfsHandle handle;
    uint32_t flags;
    int result;

    result = erfs_open(fs, (const uint8_t *)"/", strlen("/"), &handle, &size);
    EXPECT_EQ(result, ERFS_OK);    

    result = erfs_entryflags(handle, &flags);
    EXPECT_EQ(flags, ERFS_DIRECTORY);    

}

TEST(RFS, read_open_file) {
    const uint8_t * buff;
    uint32_t size;
    ErfsHandle handle;
    uint32_t flags;
    int result;

    result = erfs_open(fs, (const uint8_t *)"/src", strlen("/src"), &handle, &size);
    EXPECT_EQ(result, ERFS_OK);    
    result = erfs_entryflags(handle, &flags);
    EXPECT_EQ(flags, ERFS_DIRECTORY);  

    result = erfs_open(fs, (const uint8_t *)"/src/resource_fs.c", strlen("/src/resource_fs.c"), &handle, &size);
    EXPECT_EQ(result, ERFS_OK);    
    result = erfs_entryflags(handle, &flags);
    EXPECT_EQ(flags, ERFS_GZIPPED);    
}


} // namespace
