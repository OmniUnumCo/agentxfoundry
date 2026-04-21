cmake_minimum_required(VERSION 3.16)
project(rr_system_guard LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_executable(rr-integrityd
  src/rr_integrityd.cpp
  src/rr_util.cpp
)

add_executable(rrctl
  src/rrctl.cpp
  src/rr_util.cpp
)

target_include_directories(rr-integrityd PRIVATE include)
target_include_directories(rrctl PRIVATE include)

find_library(SQLITE3_LIB sqlite3 REQUIRED)
find_library(SECURITY_LIB Security REQUIRED)
find_library(COREFOUNDATION_LIB CoreFoundation REQUIRED)

target_link_libraries(rr-integrityd PRIVATE ${SQLITE3_LIB} ${SECURITY_LIB} ${COREFOUNDATION_LIB})
target_link_libraries(rrctl PRIVATE ${SQLITE3_LIB})
