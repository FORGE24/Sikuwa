# sikuwa/cpp_cache/CMakeLists.txt
# 使用CMake构建C++智能缓存系统

cmake_minimum_required(VERSION 3.15)
project(SmartCache LANGUAGES CXX)

# 设置C++标准
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 寻找Python库
find_package(Python3 REQUIRED COMPONENTS Development)

# 添加Python的include目录
include_directories(${Python3_INCLUDE_DIRS})

# 创建Python扩展模块
add_library(pysmartcache MODULE smart_cache.cpp pysmartcache.cpp)

# 链接Python库
target_link_libraries(pysmartcache PRIVATE ${Python3_LIBRARIES})

# 设置输出目录
set_target_properties(pysmartcache PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
)

# 如果在Windows上，确保输出为.pyd文件
if(WIN32)
    set_target_properties(pysmartcache PROPERTIES SUFFIX ".pyd")
endif()

# 创建一个简单的测试可执行文件
add_executable(test_smart_cache test_smart_cache.cpp)
target_link_libraries(test_smart_cache PRIVATE)
