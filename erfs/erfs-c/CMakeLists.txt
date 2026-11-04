cmake_minimum_required(VERSION 3.10)
project(ERFS_C)

set(CMAKE_CXX_STANDARD 17)

# ERFS C runtime library
add_library(erfsrt STATIC src/rt/resource_fs.c)
target_include_directories(erfsrt PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/src/rt)

# ERFS generator executable
add_executable(erfsgen src/gen/main.cpp src/gen/erfs_generator.cpp)

# Test: generate resources from src/ into build/test_gen/, then compile test
set(TEST_GEN_DIR ${CMAKE_CURRENT_BINARY_DIR}/test_gen)
file(MAKE_DIRECTORY ${TEST_GEN_DIR})

add_custom_command(
    OUTPUT ${TEST_GEN_DIR}/erfs_test.c ${TEST_GEN_DIR}/erfs_test.h ${TEST_GEN_DIR}/erfs_test.img
    COMMAND $<TARGET_FILE:erfsgen> --image ${CMAKE_CURRENT_SOURCE_DIR}/src test ${TEST_GEN_DIR}
    DEPENDS erfsgen
    COMMENT "Generating test resources from src/"
)

add_executable(erfs_test
    test/test_erfs.c
    src/rt/resource_fs.c
    ${TEST_GEN_DIR}/erfs_test.c
)
target_include_directories(erfs_test PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src/rt ${TEST_GEN_DIR})
add_dependencies(erfs_test erfsgen)

add_custom_command(TARGET erfs_test POST_BUILD
    COMMAND $<TARGET_FILE:erfs_test> ${TEST_GEN_DIR}/erfs_test.img
    COMMENT "Running ERFS tests..."
)
