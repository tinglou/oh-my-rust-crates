# ERFS-C

嵌入式资源文件系统 (C 语言实现)

ERFS-C 是一个轻量级的 C 库，用于将文件系统资源直接嵌入到应用程序二进制文件中，或从独立的自解释镜像文件加载。它包含两个主要组件：

- **erfsrt** - 用于读取嵌入式资源的最小 C 运行时库
- **erfsgen** - 一个 C++17 代码生成器，将目录树打包为 C 源文件和/或二进制镜像

## 特性

- 零依赖（仅标准 C/C++）
- 将整个目录树嵌入为只读文件系统
- 二分查找实现快速文件查找
- 目录遍历 API
- 两种加载模式：嵌入到二进制文件或从独立 `.img` 文件加载
- 无运行时分配 - 所有数据嵌入在只读段中
- 跨平台支持（Linux、macOS、Windows）

## 构建

```bash
mkdir build && cd build
cmake ..
make
```

这将产生：
- `liberfsrt.a` - 运行时静态库
- `erfsgen` - 代码生成器可执行文件
- `erfs_test` - 测试可执行文件（验证两种加载模式）

## 快速开始

### 1. 准备资源文件

```
my_resources/
├── index.html
├── style.css
└── images/
    └── logo.png
```

### 2. 生成嵌入式文件系统

```bash
./erfsgen my_resources myapp ./generated
```

这将创建：
- `generated/erfs_myapp.c` - 嵌入式文件系统数据
- `generated/erfs_myapp.h` - 包含访问器函数的头文件

同时生成独立二进制镜像文件：

```bash
./erfsgen --image my_resources myapp ./generated
```

额外创建：
- `generated/erfs_myapp.img` - 自解释二进制镜像

### 3. 在项目中使用

**CMakeLists.txt:**
```cmake
add_library(erfsrt STATIC path/to/erfs-c/src/rt/resource_fs.c)
target_include_directories(erfsrt PUBLIC path/to/erfs-c/src/rt)

add_executable(myapp main.c generated/erfs_myapp.c)
target_link_libraries(myapp erfsrt)
```

**main.c:**
```c
#define __ERFS_IMPL__
#include "erfs_myapp.h"
#include "resource_fs.h"
#include <stdio.h>

int main() {
    // 模式 1：嵌入式资源
    ErfsRoot fs = erfs_myapp();
    
    // 通过路径读取文件
    const uint8_t *content;
    uint32_t size;
    if (erfs_read(fs, (uint8_t*)"index.html", 10, &content, &size) == ERFS_OK) {
        printf("File size: %u\n", size);
        fwrite(content, 1, size, stdout);
    }
    
    // 列出目录内容
    ErfsHandle dir;
    if (erfs_open(fs, (uint8_t*)"images", 6, &dir, &size) == ERFS_OK) {
        for (uint32_t i = 0; i < size; i++) {
            ErfsHandle entry;
            erfs_readdir(fs, dir, i, &entry);
            const uint8_t *name;
            uint32_t name_len;
            erfs_entryname(fs, entry, &name, &name_len);
            printf("Entry: %.*s\n", name_len, name);
        }
    }
    
    return 0;
}
```

### 4. 从镜像文件加载

```c
#define __ERFS_IMPL__
#include "resource_fs.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    // 从独立镜像文件加载
    FILE* f = fopen("myapp.img", "rb");
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    void* buf = malloc(size);
    fread(buf, 1, size, f);
    fclose(f);
    
    ErfsRoot fs = erfs_from_buffer(buf, size);
    if (!fs) {
        fprintf(stderr, "Failed to load image\n");
        return 1;
    }
    
    // 使用相同的 API
    const uint8_t *content;
    uint32_t content_size;
    erfs_read(fs, (uint8_t*)"index.html", 10, &content, &content_size);
    
    free(buf);
    return 0;
}
```

### 5. 构建并运行

```bash
gcc -Ipath/to/erfs-c/src/rt -I./generated main.c generated/erfs_myapp.c path/to/erfs-c/src/rt/resource_fs.c -o myapp
./myapp
```

## API 参考

### 运行时 (resource_fs.h)

| 函数 | 说明 |
|------|------|
| `erfs_read(fs, path, path_len, out, size)` | 通过路径读取文件 |
| `erfs_open(fs, path, path_len, handle, size)` | 打开文件或目录 |
| `erfs_entryflags(handle, flags)` | 获取条目标志（文件/目录） |
| `erfs_entrysize(handle, size)` | 获取条目大小 |
| `erfs_entryname(fs, handle, out, size)` | 获取条目名称 |
| `erfs_readfile(fs, handle, out, size)` | 通过句柄读取文件 |
| `erfs_readdir(fs, dir, index, out)` | 按索引读取目录条目 |
| `erfs_travel(fs, func, ctx)` | 遍历整个文件系统 |
| `erfs_from_buffer(buffer, size)` | 从内存缓冲区加载文件系统 |

### 生成器 (erfsgen)

```bash
erfsgen [options] <src_dir> <id> <dest_dir>
```

输出格式选项 (默认: `--c`):
- `--c` - 生成 C 源文件
- `--image` - 生成二进制镜像文件

字节序选项 (仅对镜像生效，默认: 本机字节序):
- `--le` - 生成小端序镜像
- `--be` - 生成大端序镜像

`id` 必须匹配正则 `[a-z][a-z_0-9]*`。

## 生成的文件

以 `id = "myapp"` 为例：

| 文件 | 用途 |
|------|------|
| `erfs_myapp.h` | 头文件，声明 `ErfsRoot erfs_myapp(void);` |
| `erfs_myapp.c` | 嵌入式文件系统数据 + 访问器函数 |
| `erfs_le_myapp.img` | 小端序二进制镜像 (使用 `--image --le`) |
| `erfs_be_myapp.img` | 大端序二进制镜像 (使用 `--image --be`) |

## 文件格式

详见 [layout.md](layout.md) 了解二进制格式的详细文档。

## 许可证

MIT 许可证 - 详见 LICENSE 文件。

## 仓库

https://github.com/tinglou/erfs
