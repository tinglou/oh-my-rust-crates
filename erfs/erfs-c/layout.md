# ERFS Binary Format Layout

This document describes the binary format of ERFS filesystem images.

## Overview

The ERFS format is a compact, self-describing binary format for embedding filesystem resources. All offsets are relative to the `data[]` flexible array member of the header.

## Header Structure

```
Offset  Size  Field         Description
------  ----  -----         -----------
0       4     magic         Magic number: 0x45524653 ("ERFS" in little-endian)
4       4     total_size    Total size of the image in bytes
8       2     entry_count   Number of entries (files + directories)
10      2     _reserved     Reserved for future use (set to 0)
12      ...   data[]        Flexible array containing entry table, names, and file data
```

**Total header size: 12 bytes**

## Entry Structure (ErfsEntry)

Each entry is 16 bytes:

```
Offset  Size  Field         Description
------  ----  -----         -----------
0       4     data_offset   For files: offset to file content (relative to data[])
                            For directories: offset to first child entry (relative to data[])
4       4     data_size     For files: file size in bytes
                            For directories: number of child entries
8       4     name_offset   Offset to entry name (relative to data[])
12      2     name_size     Length of entry name in bytes
14      2     flags         Entry flags (ERFS_DIRECTORY = 1)
```

**Total entry size: 16 bytes**

## Data Section Layout

The `data[]` section contains three contiguous regions:

```
data[]
├── Entry Table (entry_count × 16 bytes)
│   ├── Entry[0] (root directory "/")
│   ├── Entry[1]
│   ├── Entry[2]
│   └── ...
├── Names Section
│   ├── Name[0] (root "/")
│   ├── Name[1]
│   ├── Name[2]
│   └── ...
└── File Data Section
    ├── File content for Entry[3]
    ├── File content for Entry[4]
    └── ...
```

### Entry Table

- Starts at offset 0 in `data[]`
- Contains `entry_count` entries, each 16 bytes
- Entry[0] is always the root directory "/"
- Directory children are stored contiguously

### Names Section

- Starts immediately after the entry table
- Contains all entry names concatenated
- Names are NOT null-terminated (use `name_size` to determine length)
- Names are stored in the same order as entries

### File Data Section

- Starts immediately after the names section
- Contains file contents for all regular files
- Files are stored in entry order
- Directories have no data in this section

## Directory Structure

Directories use `data_offset` and `data_size` to reference their children:

- `data_offset`: Offset (relative to `data[]`) of the first child entry
- `data_size`: Number of child entries

Example: If directory at Entry[0] has `data_offset = 160` and `data_size = 3`, its children are at:
- Entry at `data[160]` (Entry[10] if each entry is 16 bytes)
- Entry at `data[176]` (Entry[11])
- Entry at `data[192]` (Entry[12])

**Important**: Child entries must be contiguous in the entry table.

## Example Layout

For a filesystem with structure:
```
/
├── gen/
│   └── main.cpp
└── rt/
    └── resource_fs.h
```

The entry table would be:

| Index | Name | Type | data_offset | data_size | name_offset | name_size | flags |
|-------|------|------|-------------|-----------|-------------|-----------|-------|
| 0 | / | dir | 16 | 2 | 0 | 1 | 1 |
| 1 | gen | dir | 48 | 1 | 1 | 3 | 1 |
| 2 | rt | dir | 64 | 1 | 4 | 2 | 1 |
| 3 | main.cpp | file | 100 | 1234 | 7 | 8 | 0 |
| 4 | resource_fs.h | file | 1334 | 567 | 15 | 13 | 0 |

Where:
- Entry[0] (root) has children at Entry[1] and Entry[2]
- Entry[1] (gen) has child at Entry[3]
- Entry[2] (rt) has child at Entry[4]

## Binary Image Example

```
Offset  Hex Data                          Description
------  --------                          -----------
0000    53 46 52 45                       magic: "SFRE" (0x45524653)
0004    d3 68 00 00                       total_size: 26835
0008    0a 00                             entry_count: 10
000a    00 00                             _reserved: 0
000c    10 00 00 00 03 00 00 00           Entry[0]: data_offset=16, data_size=3
        a0 00 00 00 01 00 01 00           name_offset=160, name_size=1, flags=1 (dir)
001c    40 00 00 00 03 00 00 00           Entry[1]: data_offset=64, data_size=3
        a1 00 00 00 03 00 01 00           name_offset=161, name_size=3, flags=1 (dir)
...
00a0    2f 67 65 6e 72 74 ...             Names: "/genrt..."
0100    23 69 6e 63 6c 75 64 65 ...       File data: "#include..."
```

## Usage Modes

### Mode 1: Embedded in C Source

The generator creates a `.c` file with the image as a byte array:

```c
static const uint8_t erfs_myapp_data[] = "SFRE\x...";

ErfsRoot erfs_myapp(void) {
    return (ErfsRoot)erfs_myapp_data;
}
```

### Mode 2: Standalone Image File

The generator creates a `.img` file that can be loaded at runtime:

```c
void* buffer = load_file("myapp.img");
ErfsRoot fs = erfs_from_buffer(buffer, size);
```

The `erfs_from_buffer()` function validates the magic number and total size.

## Constraints

- Maximum entry count: 65535 (uint16_t)
- Maximum name length: 65535 bytes (uint16_t)
- Maximum file size: ~4GB (uint32_t)
- Maximum image size: ~4GB (uint32_t)
- All entries in a directory must be contiguous
- Entry names within a directory must be sorted for binary search

## Flags

| Flag | Value | Description |
|------|-------|-------------|
| ERFS_DIRECTORY | 1 | Entry is a directory |
| ERFS_GZIPPED | 2 | Entry is gzip compressed (reserved) |
