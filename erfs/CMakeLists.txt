

cmake_minimum_required(VERSION 3.5.1)

include(ExternalProject)

project(resource-fs)
set(CMAKE_CXX_STANDARD "17")

enable_testing()
find_package(GTest REQUIRED)


include_directories(
    erfs-rt/src
    ${CMAKE_CURRENT_BINARY_DIR}
    ${CMAKE_BINARY_DIR}/thirdparties/src/zlib
)

link_directories(
    ${CMAKE_BINARY_DIR}/thirdparties/src/zlib-build
)

# set(CMAKE_EXE_LINKER_FLAGS "-static-libstdc++")

#
# Resource Filesystem Readonly
#
set(ERFS "erfs_rt")
set(ERFS_FILES "erfs-rt/src/resource_fs.c")
add_library(${ERFS} STATIC "${ERFS_FILES}")

#
# generator 
#
set(ERFS_GEN "erfs_gen")
set(ERFS_GEN_FILES 
    erfs-gen/src/main.cpp
    erfs-gen/src/erfs_generator.cpp
    erfs-gen/src/gzip_file.cpp
    )
add_executable(${ERFS_GEN}  "${ERFS_GEN_FILES}")
target_link_libraries(${ERFS_GEN} libz.a)

#
# generate ERFS .c source file
#
function(gen_erfs_source sourcedir id target)
    add_custom_command(
        OUTPUT ${target}/erfs_${id}.c ${target}/erfs_${id}.h
        COMMAND ${ERFS_GEN} --gzip --rust ${sourcedir} ${id} ${target}
        COMMENT "Generating ERFS source file from: ${sourcedir}"
    )
endfunction()

gen_erfs_source("${CMAKE_CURRENT_SOURCE_DIR}/erfs-rt" "rfsrc" "${CMAKE_CURRENT_BINARY_DIR}")


#
# Unit test
#
set(ERFS_UT "erfs_ut")
set(ERFS_UT_FILES erfs-rt/tests/erfs_test.cpp ${CMAKE_CURRENT_BINARY_DIR}/erfs_rfsrc.c)
add_executable(${ERFS_UT} ${ERFS_UT_FILES} ${ERFS_FILES})
#target_link_libraries(${ERFS_UT}  GTest::GTest GTest::Main -lgcov)
target_link_libraries(${ERFS_UT}  GTest::GTest GTest::Main)
#set_target_properties(${ERFS_UT} PROPERTIES COMPILE_FLAGS "-fprofile-arcs -ftest-coverage")
add_test(${ERFS_UT} ${ERFS_UT})

#
#
set(TP_NAME zlib)
ExternalProject_Add(${TP_NAME}
    PREFIX                  ${PROJECT_BINARY_DIR}/thirdparties
    URL                     file://${PROJECT_SOURCE_DIR}/thirdparties/zlib-1.2.11.tar.gz

    INSTALL_COMMAND         ""
    )