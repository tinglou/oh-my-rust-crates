/**
 * @file erfs_tool.c
 * @brief ERFS 镜像文件命令行工具
 *
 * 提供 ls、cat、tree 三个子命令，用于查看镜像文件内容。
 */
#define __ERFS_IMPL__
#include "erfs_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

/**
 * @brief 加载镜像文件到内存
 *
 * @param path 镜像文件路径
 * @param out_size [out] 文件大小
 * @return 文件内容指针，失败返回 NULL
 */
static void* load_image(const char* path, uint32_t* out_size) {
    FILE* f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "error: cannot open file: %s\n", path);
        return NULL;
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (size <= 0) {
        fprintf(stderr, "error: empty or invalid file: %s\n", path);
        fclose(f);
        return NULL;
    }
    void* buf = malloc((size_t)size);
    if (!buf) {
        fprintf(stderr, "error: out of memory\n");
        fclose(f);
        return NULL;
    }
    if (fread(buf, 1, (size_t)size, f) != (size_t)size) {
        fprintf(stderr, "error: failed to read file: %s\n", path);
        free(buf);
        fclose(f);
        return NULL;
    }
    fclose(f);
    *out_size = (uint32_t)size;
    return buf;
}

/**
 * @brief 打印用法信息
 */
static void usage(const char* prog) {
    fprintf(stderr, "Usage:\n");
    fprintf(stderr, "  %s ls <dir> <img>      - list directory entries\n", prog);
    fprintf(stderr, "  %s cat <path> <img>    - print file content\n", prog);
    fprintf(stderr, "  %s tree <dir> <img>    - print directory tree\n", prog);
}

/**
 * @brief ls 命令：列出目录内容
 *
 * @param fs   文件系统句柄
 * @param path 目录路径
 */
static int cmd_ls(const ErfsRoot fs, const char* path) {
    ErfsHandle dir;
    uint32_t count;
    int rc = erfs_open(fs, (const uint8_t*)path, (uint32_t)strlen(path), &dir, &count);
    if (rc != ERFS_OK) {
        fprintf(stderr, "error: cannot open '%s': %d\n", path, rc);
        return 1;
    }
    uint32_t flags;
    erfs_entryflags(dir, &flags);
    if (!(flags & ERFS_DIRECTORY)) {
        fprintf(stderr, "error: '%s' is not a directory\n", path);
        return 1;
    }
    for (uint32_t i = 0; i < count; i++) {
        ErfsHandle entry;
        rc = erfs_readdir(fs, dir, i, &entry);
        if (rc != ERFS_OK) continue;

        const uint8_t* name;
        uint32_t name_len;
        erfs_entryname(fs, entry, &name, &name_len);

        uint32_t entry_flags;
        erfs_entryflags(entry, &entry_flags);
        char type = (entry_flags & ERFS_DIRECTORY) ? 'D' : 'F';

        uint32_t size;
        erfs_entrysize(entry, &size);

        printf("%.*s %c %u\n", name_len, name, type, size);
    }
    return 0;
}

/**
 * @brief cat 命令：输出文件内容
 *
 * @param fs   文件系统句柄
 * @param path 文件路径
 */
static int cmd_cat(const ErfsRoot fs, const char* path) {
    const uint8_t* content;
    uint32_t size;
    int rc = erfs_read(fs, (const uint8_t*)path, (uint32_t)strlen(path), &content, &size);
    if (rc != ERFS_OK) {
        fprintf(stderr, "error: cannot read '%s': %d\n", path, rc);
        return 1;
    }
    fwrite(content, 1, size, stdout);
    return 0;
}

/**
 * @brief 递归打印目录树
 *
 * @param fs     文件系统句柄
 * @param dir    当前目录句柄
 * @param indent 缩进级别
 */
static void print_tree(const ErfsRoot fs, const ErfsHandle dir, int indent) {
    uint32_t count;
    erfs_entrysize(dir, &count);

    for (uint32_t i = 0; i < count; i++) {
        ErfsHandle entry;
        int rc = erfs_readdir(fs, dir, i, &entry);
        if (rc != ERFS_OK) continue;

        const uint8_t* name;
        uint32_t name_len;
        erfs_entryname(fs, entry, &name, &name_len);

        uint32_t flags;
        erfs_entryflags(entry, &flags);
        bool is_dir = (flags & ERFS_DIRECTORY) != 0;

        for (int j = 0; j < indent; j++) {
            printf("  ");
        }
        uint32_t size;
        erfs_entrysize(entry, &size);
        printf("%.*s %c %u\n", name_len, name, is_dir ? 'D' : 'F', size);

        if (is_dir) {
            print_tree(fs, entry, indent + 1);
        }
    }
}

/**
 * @brief tree 命令：打印目录树
 *
 * @param fs   文件系统句柄
 * @param path 目录路径
 */
static int cmd_tree(const ErfsRoot fs, const char* path) {
    ErfsHandle dir;
    uint32_t count;
    int rc = erfs_open(fs, (const uint8_t*)path, (uint32_t)strlen(path), &dir, &count);
    if (rc != ERFS_OK) {
        fprintf(stderr, "error: cannot open '%s': %d\n", path, rc);
        return 1;
    }
    uint32_t flags;
    erfs_entryflags(dir, &flags);
    if (!(flags & ERFS_DIRECTORY)) {
        fprintf(stderr, "error: '%s' is not a directory\n", path);
        return 1;
    }

    const uint8_t* name;
    uint32_t name_len;
    erfs_entryname(fs, dir, &name, &name_len);
    printf("%.*s D %u\n", name_len, name, count);

    print_tree(fs, dir, 1);
    return 0;
}

int main(int argc, char** argv) {
    if (argc < 4) {
        usage(argv[0]);
        return 1;
    }

    const char* cmd = argv[1];
    const char* path = argv[2];
    const char* img_path = argv[3];

    uint32_t img_size;
    void* img_data = load_image(img_path, &img_size);
    if (!img_data) {
        return 1;
    }

    ErfsRoot fs = erfs_from_buffer(img_data, img_size);
    if (!fs) {
        fprintf(stderr, "error: invalid ERFS image: %s\n", img_path);
        free(img_data);
        return 1;
    }

    int result = 0;
    if (strcmp(cmd, "ls") == 0) {
        result = cmd_ls(fs, path);
    } else if (strcmp(cmd, "cat") == 0) {
        result = cmd_cat(fs, path);
    } else if (strcmp(cmd, "tree") == 0) {
        result = cmd_tree(fs, path);
    } else {
        fprintf(stderr, "error: unknown command: %s\n", cmd);
        usage(argv[0]);
        result = 1;
    }

    free(img_data);
    return result;
}
