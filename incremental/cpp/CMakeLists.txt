# sikuwa/incremental/cpp/CMakeLists.txt
cmake_minimum_required(VERSION 3.14)
project(incremental_engine)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# 查找 Python 和 pybind11
find_package(Python3 COMPONENTS Interpreter Development REQUIRED)
find_package(pybind11 CONFIG QUIET)

if(NOT pybind11_FOUND)
    # 如果没有安装 pybind11，使用 FetchContent 下载
    include(FetchContent)
    FetchContent_Declare(
        pybind11
        GIT_REPOSITORY https://github.com/pybind/pybind11.git
        GIT_TAG v2.11.1
    )
    FetchContent_MakeAvailable(pybind11)
endif()

# 源文件
set(SOURCES
    incremental_core.cpp
    pybind_incremental.cpp
)

set(HEADERS
    incremental_core.h
)

# 创建 Python 模块
pybind11_add_module(incremental_engine ${SOURCES} ${HEADERS})

# 优化选项
target_compile_options(incremental_engine PRIVATE
    $<$<CXX_COMPILER_ID:GNU>:-O3 -Wall -Wextra>
    $<$<CXX_COMPILER_ID:Clang>:-O3 -Wall -Wextra>
    $<$<CXX_COMPILER_ID:MSVC>:/O2 /W4>
)

# 安装
install(TARGETS incremental_engine DESTINATION .)
