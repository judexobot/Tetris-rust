cmake_minimum_required(VERSION 3.22.1)
project(TetrisRustAndroid LANGUAGES C CXX)

add_library(tetris_core STATIC IMPORTED)
set_target_properties(tetris_core PROPERTIES
    IMPORTED_LOCATION "${CMAKE_CURRENT_SOURCE_DIR}/prebuilt/${ANDROID_ABI}/libtetris_nes_core.a")
add_library(tetris_jni SHARED native_bridge.cpp)
target_compile_features(tetris_jni PRIVATE cxx_std_17)
target_link_options(tetris_jni PRIVATE "-Wl,-z,max-page-size=16384")
target_link_libraries(tetris_jni PRIVATE tetris_core log android)
