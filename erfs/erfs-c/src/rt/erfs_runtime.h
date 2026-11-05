/**
 * @file erfs_runtime.h
 * @brief ERFS (Embedded Resource File System) 运行时库
 *
 * ERFS 是一个轻量级的嵌入式文件系统库，用于将目录树打包为只读文件系统，
 * 直接嵌入到应用程序二进制文件中，或从独立的自解释镜像文件加载。
 *
 * @note 使用本 API 前需定义 @c __ERFS_IMPL__ 宏以启用完整类型定义。
 *
 * @see layout.md 了解二进制格式详情
 */
#pragma once

#if defined(__ERFS_IMPL__)
#include <stdint.h>
#if defined(__cplusplus)
extern "C" {
#endif

/**
 * @defgroup Magic 魔数常量
 * @brief 用于标识镜像字节序的魔数
 * @{
 */

/** @brief 小端序镜像魔数 (字节序列: ERFS) */
#define ERFS_MAGIC_LE 0x45524653

/** @brief 大端序镜像魔数 (字节序列: SFRE) */
#define ERFS_MAGIC_BE 0x53465245

/** @} */

/**
 * @struct ErfsEntry
 * @brief 文件系统条目 (文件或目录)
 *
 * 每个条目固定 16 字节，所有偏移量均相对于 @c ErfsFileSystem.data[] 。
 *
 * @note 结构体字段已按自然对齐排列，无填充字节。
 */
typedef struct {
    uint32_t data_offset;  /**< 文件: 内容偏移量; 目录: 首个子条目偏移量 */
    uint32_t data_size;    /**< 文件: 内容大小(字节); 目录: 子条目数量 */
    uint32_t name_offset;  /**< 条目名称在 data[] 中的偏移量 */
    uint16_t name_size;    /**< 条目名称长度(字节)，不以 null 结尾 */
    uint16_t flags;        /**< 条目标志，参见 @ref ErfsEntryFlags */
} ErfsEntry;

/**
 * @typedef ErfsHandle
 * @brief 文件系统条目句柄 (指向 ErfsEntry 的常量指针)
 */
typedef const ErfsEntry* ErfsHandle;

/**
 * @struct ErfsFileSystem
 * @brief ERFS 文件系统头部
 *
 * 整个镜像的头部结构，固定 12 字节。
 * @c data[] 柔性数组包含三个连续区域：
 * - 条目表 (entry_count × 16 字节)
 * - 名称区 (所有条目名称连续存储)
 * - 文件数据区 (所有文件内容连续存储)
 */
typedef struct {
    uint32_t magic;        /**< 魔数，用于标识字节序: ERFS_MAGIC_LE 或 ERFS_MAGIC_BE */
    uint32_t total_size;   /**< 镜像总大小(字节) */
    uint16_t entry_count;  /**< 条目总数 (文件 + 目录) */
    uint16_t _reserved;    /**< 保留字段，当前为 0 */
    uint8_t  data[0];      /**< 柔性数组: 条目表 + 名称 + 文件数据 */
} ErfsFileSystem;

/**
 * @typedef ErfsRoot
 * @brief 文件系统根句柄 (指向 ErfsFileSystem 的常量指针)
 */
typedef const ErfsFileSystem* ErfsRoot;

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

/**
 * @defgroup Flags 条目标志
 * @brief ErfsEntry.flags 的可选值
 * @{
 */
enum ErfsEntryFlags {
    ERFS_DIRECTORY = 1,  /**< 条目为目录 */
    ERFS_GZIPPED   = 2,  /**< 条目内容经 gzip 压缩 (保留) */
};
/** @} */

/**
 * @defgroup Status 状态码
 * @brief API 函数返回值
 * @{
 */
enum ErfsStatusCode {
    ERFS_OK            =  0,  /**< 操作成功 */
    ERFS_INVALID_INPUT = -1,  /**< 输入参数无效 (空指针等) */
    ERFS_NOT_FOUND     = -2,  /**< 路径不存在 */
    ERFS_NOT_FILE      = -3,  /**< 条目不是文件 */
    ERFS_NOT_DIRECTORY = -4,  /**< 条目不是目录 */
    ERFS_OUTOF_BOUND   = -5,  /**< 索引越界 */
    ERFS_INVALID_MAGIC = -6,  /**< 魔数不匹配 */
    ERFS_INVALID_SIZE  = -7,  /**< 大小不匹配 */
};
/** @} */

/**
 * @brief 通过路径读取文件内容
 *
 * 根据路径打开文件并返回其内容的指针和大小。
 * 返回的指针直接指向嵌入数据，无需释放。
 *
 * @param fs       文件系统根句柄
 * @param path     文件路径 (如 "/dir/file.txt")
 * @param path_len 路径长度(字节)
 * @param out      [out] 文件内容指针
 * @param size     [out] 文件大小(字节)
 * @return @ref ERFS_OK 成功，其他值表示错误
 *
 * @retval ERFS_OK          成功
 * @retval ERFS_INVALID_INPUT 参数为空
 * @retval ERFS_NOT_FOUND   路径不存在
 * @retval ERFS_NOT_FILE    路径指向目录而非文件
 *
 * @code{.c}
 * const uint8_t *content;
 * uint32_t size;
 * if (erfs_read(fs, (uint8_t*)"/index.html", 11, &content, &size) == ERFS_OK) {
 *     fwrite(content, 1, size, stdout);
 * }
 * @endcode
 */
int erfs_read(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, const uint8_t **out, uint32_t *size);

/**
 * @brief 打开文件或目录
 *
 * 根据路径查找条目，返回句柄和大小信息。
 * 对于文件，size 为文件大小；对于目录，size 为子条目数量。
 *
 * @param fs       文件系统根句柄
 * @param path     路径 (支持 "/" 前缀，如 "/gen/main.cpp")
 * @param path_len 路径长度(字节)
 * @param out      [out] 条目句柄
 * @param size     [out] 文件大小或目录子条目数
 * @return @ref ERFS_OK 成功，其他值表示错误
 *
 * @note 不支持 "/../" 或 "/./" 相对路径
 */
int erfs_open(const ErfsRoot fs, const uint8_t *path, uint32_t path_len, ErfsHandle *out, uint32_t *size);

/**
 * @brief 获取条目标志
 *
 * @param handle 条目句柄
 * @param flags  [out] 标志值，参见 @ref ErfsEntryFlags
 * @return @ref ERFS_OK 成功
 */
int erfs_entryflags(const ErfsHandle handle, uint32_t *flags);

/**
 * @brief 获取条目大小
 *
 * @param handle 条目句柄
 * @param size   [out] 文件大小(字节)或目录子条目数
 * @return @ref ERFS_OK 成功
 */
int erfs_entrysize(const ErfsHandle handle, uint32_t *size);

/**
 * @brief 获取条目名称
 *
 * @param fs     文件系统根句柄
 * @param handle 条目句柄
 * @param out    [out] 名称指针 (不以 null 结尾)
 * @param size   [out] 名称长度(字节)
 * @return @ref ERFS_OK 成功
 */
int erfs_entryname(const ErfsRoot fs, const ErfsHandle handle, const uint8_t **out, uint32_t *size);

/**
 * @brief 通过句柄读取文件内容
 *
 * 与 erfs_read() 类似，但使用已打开的句柄而非路径。
 *
 * @param fs     文件系统根句柄
 * @param handle 文件条目句柄 (必须是文件，非目录)
 * @param out    [out] 文件内容指针
 * @param size   [out] 文件大小(字节)
 * @return @ref ERFS_OK 成功
 * @retval ERFS_NOT_FILE 句柄指向目录
 */
int erfs_readfile(const ErfsRoot fs, const ErfsHandle handle, const uint8_t **out, uint32_t *size);

/**
 * @brief 读取目录中的指定条目
 *
 * @param fs     文件系统根句柄
 * @param dir    目录条目句柄
 * @param index  子条目索引 (从 0 开始)
 * @param out    [out] 子条目句柄
 * @return @ref ERFS_OK 成功
 * @retval ERFS_NOT_DIRECTORY 句柄不是目录
 * @retval ERFS_OUTOF_BOUND   索引越界
 *
 * @code{.c}
 * ErfsHandle dir;
 * uint32_t count;
 * erfs_open(fs, (uint8_t*)"/images", 7, &dir, &count);
 * for (uint32_t i = 0; i < count; i++) {
 *     ErfsHandle entry;
 *     erfs_readdir(fs, dir, i, &entry);
 *     // 处理 entry...
 * }
 * @endcode
 */
int erfs_readdir(const ErfsRoot fs, const ErfsHandle dir, uint32_t index, ErfsHandle *out);

/**
 * @defgroup Travel 遍历类型
 * @brief erfs_travel() 回调的事件类型
 * @{
 */
enum ErfsTravelType {
    ERFS_TRAVEL_DIR_ENTER,  /**< 进入目录 (在遍历子条目之前) */
    ERFS_TRAVEL_DIR_LEAVE,  /**< 离开目录 (在遍历子条目之后) */
    ERFS_TRAVEL_FILE,       /**< 访问文件 */
};
/** @} */

/**
 * @typedef ErfsVisitFn
 * @brief 遍历回调函数类型
 *
 * @param fs    文件系统根句柄
 * @param entry 当前条目
 * @param type  事件类型
 * @param ctx   用户上下文
 * @return 0 继续遍历，非 0 中止遍历
 */
typedef int (*ErfsVisitFn) (const ErfsRoot fs, const ErfsHandle entry, enum ErfsTravelType type, void* ctx);

/**
 * @brief 深度优先遍历整个文件系统
 *
 * 从根目录开始，按深度优先顺序遍历所有条目。
 * 对于目录，先触发 DIR_ENTER，然后递归遍历子条目，最后触发 DIR_LEAVE。
 * 对于文件，触发 FILE 事件。
 *
 * @param fs   文件系统根句柄
 * @param func 回调函数
 * @param ctx  传递给回调的用户上下文
 * @return @ref ERFS_OK 成功，或回调返回的非零值
 *
 * @code{.c}
 * int list_files(const ErfsRoot fs, const ErfsHandle entry,
 *                enum ErfsTravelType type, void* ctx) {
 *     if (type == ERFS_TRAVEL_FILE) {
 *         const uint8_t *name;
 *         uint32_t name_len;
 *         erfs_entryname(fs, entry, &name, &name_len);
 *         printf("%.*s\n", name_len, name);
 *     }
 *     return 0;
 * }
 * erfs_travel(fs, list_files, NULL);
 * @endcode
 */
int erfs_travel(const ErfsRoot fs, ErfsVisitFn func, void* ctx);

/**
 * @brief 从内存缓冲区加载文件系统
 *
 * 验证缓冲区的魔数和大小，返回可用的文件系统句柄。
 * 仅接受与本机字节序匹配的镜像。
 *
 * @param buffer 缓冲区指针 (需 4 字节对齐)
 * @param size   缓冲区大小(字节)
 * @return 文件系统句柄，失败返回 NULL
 *
 * @retval NULL 缓冲区无效、魔数不匹配或大小不一致
 *
 * @code{.c}
 * void* buf = load_file("resources.img");
 * uint32_t size = get_file_size();
 * ErfsRoot fs = erfs_from_buffer(buf, size);
 * if (fs) {
 *     // 使用 fs...
 * }
 * @endcode
 */
ErfsRoot erfs_from_buffer(const void* buffer, uint32_t size);

#if defined(__cplusplus)
}
#endif
