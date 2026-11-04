# ERFS 二进制格式布局

本文档描述 ERFS 文件系统镜像的二进制格式。

## 概述

ERFS 格式是一种紧凑的、自描述的二进制格式，用于嵌入文件系统资源。所有偏移量均相对于头部的 `data[]` 柔性数组成员。

## 头部结构

```
偏移量  大小  字段          描述
------  ----  -----         -----------
0       4     magic         魔数: 0x45524653 (小端序的 "ERFS")
4       4     total_size    镜像总字节数
8       2     entry_count   条目数量 (文件 + 目录)
10      2     _reserved     保留字段 (设为 0)
12      ...   data[]        柔性数组，包含条目表、名称和文件数据
```

**头部总大小: 12 字节**

## 条目结构 (ErfsEntry)

每个条目 16 字节:

```
偏移量  大小  字段          描述
------  ----  -----         -----------
0       4     data_offset   文件: 文件内容的偏移量 (相对于 data[])
                            目录: 第一个子条目的偏移量 (相对于 data[])
4       4     data_size     文件: 文件大小 (字节)
                            目录: 子条目数量
8       4     name_offset   条目名称的偏移量 (相对于 data[])
12      2     name_size     名称长度 (字节)
14      2     flags         条目标志 (ERFS_DIRECTORY = 1)
```

**条目总大小: 16 字节**

## 数据区布局

`data[]` 区段包含三个连续的区域:

```
data[]
├── 条目表 (entry_count × 16 字节)
│   ├── Entry[0] (根目录 "/")
│   ├── Entry[1]
│   ├── Entry[2]
│   └── ...
├── 名称区
│   ├── Name[0] (根目录 "/")
│   ├── Name[1]
│   ├── Name[2]
│   └── ...
└── 文件数据区
    ├── Entry[3] 的文件内容
    ├── Entry[4] 的文件内容
    └── ...
```

### 条目表

- 从 `data[]` 的偏移量 0 开始
- 包含 `entry_count` 个条目，每个 16 字节
- Entry[0] 始终是根目录 "/"
- 目录的子条目必须连续存储

### 名称区

- 紧跟在条目表之后
- 包含所有条目名称的连续拼接
- 名称不以 null 结尾 (使用 `name_size` 确定长度)
- 名称按条目顺序存储

### 文件数据区

- 紧跟在名称区之后
- 包含所有常规文件的内容
- 文件按条目顺序存储
- 目录在此区域没有数据

## 目录结构

目录使用 `data_offset` 和 `data_size` 引用其子条目:

- `data_offset`: 第一个子条目的偏移量 (相对于 `data[]`)
- `data_size`: 子条目数量

示例: 如果 Entry[0] 的目录有 `data_offset = 160` 和 `data_size = 3`，其子条目位于:
- `data[160]` 处的条目 (如果每个条目 16 字节，则为 Entry[10])
- `data[176]` 处的条目 (Entry[11])
- `data[192]` 处的条目 (Entry[12])

**重要**: 子条目在条目表中必须连续。

## 示例布局

对于以下结构的文件系统:
```
/
├── gen/
│   └── main.cpp
└── rt/
    └── resource_fs.h
```

条目表将是:

| 索引 | 名称 | 类型 | data_offset | data_size | name_offset | name_size | flags |
|------|------|------|-------------|-----------|-------------|-----------|-------|
| 0 | / | 目录 | 16 | 2 | 0 | 1 | 1 |
| 1 | gen | 目录 | 48 | 1 | 1 | 3 | 1 |
| 2 | rt | 目录 | 64 | 1 | 4 | 2 | 1 |
| 3 | main.cpp | 文件 | 100 | 1234 | 7 | 8 | 0 |
| 4 | resource_fs.h | 文件 | 1334 | 567 | 15 | 13 | 0 |

其中:
- Entry[0] (根目录) 的子条目在 Entry[1] 和 Entry[2]
- Entry[1] (gen) 的子条目在 Entry[3]
- Entry[2] (rt) 的子条目在 Entry[4]

## 二进制镜像示例

```
偏移量  十六进制数据                        描述
------  --------                          -----------
0000    53 46 52 45                       magic: "SFRE" (0x45524653)
0004    d3 68 00 00                       total_size: 26835
0008    0a 00                             entry_count: 10
000a    00 00                             _reserved: 0
000c    10 00 00 00 03 00 00 00           Entry[0]: data_offset=16, data_size=3
        a0 00 00 00 01 00 01 00           name_offset=160, name_size=1, flags=1 (目录)
001c    40 00 00 00 03 00 00 00           Entry[1]: data_offset=64, data_size=3
        a1 00 00 00 03 00 01 00           name_offset=161, name_size=3, flags=1 (目录)
...
00a0    2f 67 65 6e 72 74 ...             名称: "/genrt..."
0100    23 69 6e 63 6c 75 64 65 ...       文件数据: "#include..."
```

## 使用模式

### 模式 1: 嵌入 C 源码

生成器创建一个 `.c` 文件，将镜像作为字节数组:

```c
static const uint8_t erfs_myapp_data[] = "SFRE\x...";

ErfsRoot erfs_myapp(void) {
    return (ErfsRoot)erfs_myapp_data;
}
```

### 模式 2: 独立镜像文件

生成器创建一个 `.img` 文件，可在运行时加载:

```c
void* buffer = load_file("myapp.img");
ErfsRoot fs = erfs_from_buffer(buffer, size);
```

`erfs_from_buffer()` 函数会验证魔数和总大小。

## 约束限制

- 最大条目数: 65535 (uint16_t)
- 最大名称长度: 65535 字节 (uint16_t)
- 最大文件大小: ~4GB (uint32_t)
- 最大镜像大小: ~4GB (uint32_t)
- 目录中的所有条目必须连续
- 目录内的条目名称必须排序以支持二分查找

## 标志位

| 标志 | 值 | 描述 |
|------|-------|-------------|
| ERFS_DIRECTORY | 1 | 条目是目录 |
| ERFS_GZIPPED | 2 | 条目使用 gzip 压缩 (保留) |
