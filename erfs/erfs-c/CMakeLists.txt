cmake_minimum_required(VERSION 3.10)
project(ERFS_C)

set(CMAKE_CXX_STANDARD 17)

# ERFS C runtime library
add_library(erfsrt STATIC src/rt/resource_fs.c)

# PUBLIC 表示：ERFS 自己编译时需要这个路径，且依赖 mylib 的目标（如 myapp）也需要这个路径
target_include_directories(erfsrt PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/src/rt)

# ERFS 构建工具可执行文件
add_executable(erfsgen src/gen/main.cpp src/gen/gzip_file.cpp src/gen/erfs_generator.cpp)

