/**
 * @file erfs_generator.h
 * @brief ERFS 代码生成器接口
 *
 * 本模块提供将目录树打包为 ERFS 格式的功能，
 * 可生成嵌入式的 C 源文件和/或独立的二进制镜像文件。
 */
#pragma once

#if defined(__cplusplus)
extern "C" {
#endif

/**
 * @defgroup GenOptions 生成选项
 * @brief erfs_generate() 的选项标志
 * @{
 */
enum ErfsGenOption {
    ERFS_GEN_C     = 1,   /**< 生成 C 源文件 (.c) */
    ERFS_GEN_IMAGE = 2,   /**< 生成二进制镜像文件 (.img) */
    ERFS_GEN_LE    = 4,   /**< 生成小端序镜像 */
    ERFS_GEN_BE    = 8,   /**< 生成大端序镜像 */
};
/** @} */

/**
 * @defgroup GenStatus 生成器状态码
 * @brief erfs_generate() 的返回值
 * @{
 */
enum ErfsGenStatusCode {
    ERFS_GEN_OK            =   0,  /**< 操作成功 */
    ERFS_INVALID_INPUT     =  -1,  /**< 输入参数无效 */
    ERFS_NOT_FOUND         =  -2,  /**< 源路径不存在 */
    ERFS_NOT_FILE          =  -3,  /**< 源路径不是文件或目录 */
    ERFS_NOT_DIRECTORY     =  -4,  /**< 源路径不是目录 */
    ERFS_OUTOF_BOUND       =  -5,  /**< 索引越界 */

    ERFS_SOURCE_TOO_LARGE  = -100, /**< 源数据过大（如条目数超过 65535） */
    ERFS_TARGET_NOT_EXIST  = -101, /**< 目标目录不存在 */
    ERFS_INVALID_ID        = -102, /**< ID 格式无效 */
    ERFS_INVALID_OPTION    = -103, /**< 选项组合无效 */
};
/** @} */

/**
 * @brief 生成 ERFS 源文件和/或镜像文件
 *
 * 将指定目录或文件打包为 ERFS 格式，根据选项生成：
 * - C 源文件: @c erfs_<id>.c (包含嵌入式数据)
 * - 头文件: @c erfs_<id>.h (始终生成)
 * - 镜像文件: @c erfs_le_<id>.img 和/或 @c erfs_be_<id>.img
 *
 * @param path       源目录或文件路径
 * @param id         资源标识符，格式: [a-z][a-z_0-9]*
 * @param options    选项组合，参见 @ref GenOptions
 * @param target_dir 输出目录路径
 * @return @ref ERFS_GEN_OK 成功，其他值表示错误
 *
 * @par 选项说明
 * - 未指定 @c ERFS_GEN_C 和 @c ERFS_GEN_IMAGE 时，默认生成 C 源文件
 * - 指定 @c ERFS_GEN_IMAGE 但未指定字节序时，生成与本机字节序匹配的镜像
 * - 同时指定 @c ERFS_GEN_LE 和 @c ERFS_GEN_BE 时，生成两种镜像
 *
 * @par 生成的 C 文件结构
 * 生成的 C 文件使用条件编译 (@c \#if __BYTE_ORDER__) 选择适当的
 * 文件头和条目表字节序，名称和文件数据部分共享同一份。
 *
 * @code{.cpp}
 * // 生成 C 源文件 + 本机字节序镜像
 * erfs_generate("/path/to/resources", "myapp",
 *               ERFS_GEN_C | ERFS_GEN_IMAGE, "/output");
 *
 * // 生成 LE 和 BE 两种镜像
 * erfs_generate("/path/to/resources", "myapp",
 *               ERFS_GEN_IMAGE | ERFS_GEN_LE | ERFS_GEN_BE, "/output");
 * @endcode
 */
int erfs_generate(const char *path, const char *id, int options, const char *target_dir);

#if defined(__cplusplus)
}
#endif
