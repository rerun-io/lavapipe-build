# Lavapipe for macOS (arm64)

Pre-built [lavapipe](https://docs.mesa3d.org/drivers/llvmpipe.html) — Mesa's software Vulkan driver — for macOS Apple Silicon.

Lavapipe implements Vulkan in software using LLVM's JIT, with no GPU required. Useful for CI, testing, and headless rendering on machines without GPU access.

## Download

Grab `lavapipe-macos-arm64.tar.gz` from the [Releases](../../releases) page.

## Usage

```bash
tar xzf lavapipe-macos-arm64.tar.gz
export VK_DRIVER_FILES=$(pwd)/lavapipe-macos-arm64/share/vulkan/icd.d/lvp_icd.aarch64.json
vulkaninfo --summary
```

You can move the extracted directory anywhere — all library paths are relative.

## What's included

- `lib/libvulkan_lvp.dylib` — the lavapipe Vulkan driver
- `lib/libLLVM.dylib`, `lib/libzstd.1.dylib`, `lib/libSPIRV-Tools.dylib` — bundled dependencies
- `share/vulkan/icd.d/lvp_icd.aarch64.json` — ICD manifest

## Building from source

Requirements: macOS arm64 with [Homebrew](https://brew.sh).

```bash
./build.sh
```

The script installs dependencies via Homebrew, clones Mesa, builds lavapipe, bundles dylibs, and produces `lavapipe-macos-arm64.tar.gz`.
