cmake_minimum_required(VERSION 3.14)
project(lab02)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
add_subdirectory(json)
add_subdirectory(cpp-httplib)

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -Wpedantic -Werror -Wsign-compare")
set(CMAKE_CXX_FLAGS_DEBUG "-g -D_DEBUG")

add_executable(lab02 main.cpp server.cpp)
target_link_libraries(lab02 PRIVATE nlohmann_json::nlohmann_json)
target_link_libraries(lab02 PRIVATE httplib::httplib)
target_include_directories(lab02 PUBLIC include cpp-httplib json/single_include/nlohmann)