# ERFS-C

Embedded Resource File System (C Implementation)

ERFS-C is a lightweight C library for embedding filesystem resources directly into your application binary. It consists of:

- **erfsrt** - A minimal C runtime library for reading embedded resources
- **erfsgen** - A C++17 code generator that packages a directory tree into C source files

## Features

- Zero dependencies (standard C/C++ only)
- Embed entire directory trees as read-only filesystems
- Binary search for fast file lookup
- Directory traversal API
- Optional Rust FFI bindings generation
- No runtime allocations - all data embedded in read-only section
- Cross-platform (Linux, macOS, Windows)

## Building

```bash
mkdir build && cd build
cmake ..
make
```

This produces:
- `liberfsrt.a` - Static library for the runtime
- `erfsgen` - Code generator executable

## Quick Start

### 1. Prepare your resources

```
my_resources/
├── index.html
├── style.css
└── images/
    └── logo.png
```

### 2. Generate embedded filesystem

```bash
./erfsgen my_resources myapp ./generated
```

This creates:
- `generated/erfs_myapp.c` - Embedded filesystem data
- `generated/erfs_myapp.h` - Header with accessor function

### 3. Include in your project

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
    ErfsRoot fs = erfs_gen_myapp();
    
    // Read a file
    const uint8_t *content;
    uint32_t size;
    if (erfs_read(fs, (uint8_t*)"index.html", 10, &content, &size) == ERFS_OK) {
        printf("File size: %u\n", size);
        fwrite(content, 1, size, stdout);
    }
    
    // List directory contents
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

### 4. Build and run

```bash
gcc -Ipath/to/erfs-c/src/rt -I./generated main.c generated/erfs_myapp.c path/to/erfs-c/src/rt/resource_fs.c -o myapp
./myapp
```

## API Reference

### Runtime (resource_fs.h)

| Function | Description |
|----------|-------------|
| `erfs_read(fs, path, path_len, out, size)` | Read file by path |
| `erfs_open(fs, path, path_len, handle, size)` | Open file or directory |
| `erfs_entryflags(handle, flags)` | Get entry flags (file/dir) |
| `erfs_entrysize(handle, size)` | Get entry size |
| `erfs_entryname(fs, handle, out, size)` | Get entry name |
| `erfs_readfile(fs, handle, out, size)` | Read file by handle |
| `erfs_readdir(fs, dir, index, out)` | Read directory entry by index |
| `erfs_travel(fs, func, ctx)` | Traverse entire filesystem |

### Generator (erfsgen)

```bash
erfsgen [options] <src_dir> <id> <dest_dir>
```

Options:
- `--rust` - Generate Rust FFI bindings (`.rs` file)

The `id` must match regex `[a-z][a-z_0-9]*`.

## Generated Files

For `id = "myapp"`:

| File | Purpose |
|------|---------|
| `erfs_myapp.h` | Header with `ErfsRoot erfs_gen_myapp();` |
| `erfs_myapp.c` | Embedded filesystem data + accessor |
| `erfs_myapp.rs` | Rust FFI bindings (with `--rust`) |

## License

MIT License - See LICENSE file for details.

## Repository

https://github.com/tinglou/erfs