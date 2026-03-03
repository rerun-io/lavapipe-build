# Lavapipe for macOS (arm64)

Pre-built [lavapipe](https://docs.mesa3d.org/drivers/llvmpipe.html) — Mesa's software Vulkan driver — for macOS Apple Silicon.

Lavapipe implements Vulkan in software using LLVM's JIT, with no GPU required. Useful for CI, testing, and headless rendering on machines without GPU access.

## Download

Grab `lavapipe-macos-arm64.tar.gz` from the [Releases](../../releases) page.

## Usage

```bash
tar xzf lavapipe-macos-arm64.tar.gz
export VK_DRIVER_FILES=$(pwd)/lavapipe-macos-arm64/lvp_icd.aarch64.json
vulkaninfo --summary
```

You can move the extracted directory anywhere — the ICD manifest references the driver via a relative path.

## What's included

- `libvulkan_lvp.dylib` — the lavapipe Vulkan driver
- `lvp_icd.aarch64.json` — ICD manifest

Runtime dependencies (`llvm`, `zstd`, `spirv-tools`) must be installed separately (e.g. via Homebrew).

## Building from source

Requirements: macOS arm64 with [Homebrew](https://brew.sh).

```bash
./build.sh
```

The script installs dependencies via Homebrew, clones Mesa, builds lavapipe, and produces `lavapipe-macos-arm64.tar.gz`.
