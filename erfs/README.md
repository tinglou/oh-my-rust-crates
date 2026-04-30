# Embedded Resource Filesystem (C/Rust)

This project allow application written in C/C++ or rust to embed required resource files into executable binaries.

There are 2 components:

* A command-line tool named `erfs_gen` can travel a resource directory and generate .c/.h source file which should be added into your project.
* A runtime library in pure C or Rust lib crate to access the content of resources.

# Usage

## Command-Line Interface

Usage is described as below:

```txt
$./erfs-gen
Usage: ./erfs-gen [options] <src_dir> <id> <dest_dir>
Options:
  --gzip      compress file if needed.
  --rust      generate rust binding codes.

where,
<src_dir>: point to the top level directory contains resources.
<id>: the identity of the resource file system, and a executable may have multiple ERFS instances.
<dest_dir>: to specify where the source files are generated 
```

## C developer

### Code generation

A cmake function is useful to generate the files duration building time.

```cmake
function(gen_erfs_source sourcedir id target)
    add_custom_command(
        OUTPUT ${target}/erfs_${id}.c ${target}/erfs_${id}.h
        COMMAND ${ERFS_GEN} --gzip --rust ${sourcedir} ${id} ${target}
        COMMENT "Generating ERFS source file from: ${sourcedir}"
    )
endfunction()

gen_erfs_source("${CMAKE_CURRENT_SOURCE_DIR}/erfs-rt" "rfsrc" "${CMAKE_CURRENT_BINARY_DIR}")
```

Please refer to the `CMakeList.txt` for detail.

### C API

Please refer to the header file (`erfs-rt/src/resource_fs.h`) and UT example(`erfs-rt/tests/erfs_test.cpp`) for detail.

## Rust developer

### Rust code generation

A build.rs is required to generate the .c/.h/.rs files and compile the c files. Please refer to `erfs-example/build.rs` for detail.

### Rust API

Please refer to rust wrapper (`erfs-rt/src/lib.rs`) and UT example(`erfs-example/src/main.rs`) for detail.

# TODO

* symbolic/hard link support
* pure Rust implementation for runtime library and code generator.
