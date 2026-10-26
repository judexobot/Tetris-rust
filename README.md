# Tetris NES — Rust Native Android 0.2.1

A native Rust game engine and native Android app (Kotlin View + JNI + Rust staticlib), **no NES CPU emulation, no ROM requirement, no WebView and no audio**. Adapted from the TetrisNESDisasm archive provided by the owner.

## Features

- Original 10x20 game field, all 7 tetrominoes with 19 NES rotation states, no wall kicks
- NES gravity speeds, scoring, levels, soft drop, line clears and previews
- Game A / Game B, randomized prefilled garbage, Type-B height and goal bonus
- Saved personal bests for each mode, line-clear counters, pause and game over
- NES source tile graphics, portrait/landscape rendering, multitouch and gamepad buttons
- Hidden NES 10–19 levels: **hold A and press START** on level selection
- Audio intentionally not implemented

## Build

GitHub Actions automatically installs the Android SDK/NDK and Rust. Open the latest green run under **Actions → Android Rust APK (ARM64)** and download artifact **TetrisNESRust-v0.2.1-arm64-apk**. Run `cargo test --workspace --locked` for unit tests locally. An APK builds in release mode with debug-test signature unless four optional `ANDROID_*` keystore secrets are configured (different action runners may produce different debug signatures).

The initial GitHub upload may use an embedded compressed bootstrap archive. The workflow unpacks the archive before tests and APK assembly; on a successful build it also attempts to commit the expanded source back to the repo. If that write is blocked, a usable APK artifact can still be produced.

## Unfinished / differences

Not binary-identical to original, RNG is a native approximation, game B garbage differs, no NES cutscene/demo/high-score name entry, and audio disabled by request. Sprite assets from original Nintendo game remain copyrighted by their owner; MIT only applies to our new code.
