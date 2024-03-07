
cmake_minimum_required(VERSION 3.14)
project(lab03)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -Wpedantic -Werror -Wsign-compare")
set(CMAKE_CXX_FLAGS_DEBUG "-g -D_DEBUG")

find_package(Boost 1.74 COMPONENTS REQUIRED )
include_directories( ${Boost_INCLUDE_DIR} )
add_executable(server src/server.cpp)
add_executable(client src/client.cpp)
target_link_libraries(server LINK_PUBLIC ${Boost_LIBRARIES})
target_link_libraries(client LINK_PUBLIC ${Boost_LIBRARIES})

