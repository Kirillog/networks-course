
cmake_minimum_required(VERSION 3.14)
project(lab04)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wextra -Wpedantic -Werror -Wsign-compare")
set(CMAKE_CXX_FLAGS_DEBUG "-g -D_DEBUG")

find_package(Boost 1.74 COMPONENTS REQUIRED )
include_directories( ${Boost_INCLUDE_DIR})
add_executable(proxy_server src/proxy_server.cpp)
target_include_directories(proxy_server PUBLIC include)
target_link_libraries(proxy_server LINK_PUBLIC ${Boost_LIBRARIES})

