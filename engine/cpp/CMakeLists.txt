cmake_minimum_required(VERSION 3.20)

add_library(coreverse_cpp INTERFACE)

target_include_directories(coreverse_cpp INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/shared>
)
