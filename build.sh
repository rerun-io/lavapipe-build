#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORK_DIR="$SCRIPT_DIR/_work"
INSTALL_DIR="$WORK_DIR/install"
MESA_DIR="$WORK_DIR/mesa"
VENV_DIR="$WORK_DIR/.venv"
ARTIFACT_NAME="lavapipe-macos-arm64"

echo "==> Installing Homebrew dependencies"
brew install meson ninja llvm bison flex glslang spirv-tools vulkan-loader vulkan-headers

echo "==> Setting up Python venv"
python3 -m venv "$VENV_DIR"
source "$VENV_DIR/bin/activate"
pip install mako packaging pyyaml

echo "==> Cloning Mesa"
rm -rf "$MESA_DIR"
git clone --depth 1 https://gitlab.freedesktop.org/mesa/mesa.git "$MESA_DIR"

echo "==> Writing native.ini"
cat > "$MESA_DIR/native.ini" <<'INI'
[binaries]
bison = '/opt/homebrew/opt/bison/bin/bison'
llvm-config = '/opt/homebrew/opt/llvm/bin/llvm-config'
INI

echo "==> Configuring Mesa with meson"
export PATH="/opt/homebrew/opt/llvm/bin:/opt/homebrew/opt/bison/bin:/opt/homebrew/opt/flex/bin:$PATH"

meson setup "$MESA_DIR/build" "$MESA_DIR" \
    --native-file "$MESA_DIR/native.ini" \
    -Dprefix="$INSTALL_DIR" \
    -Dbuildtype=release \
    -Dgallium-drivers=llvmpipe \
    -Dvulkan-drivers=swrast \
    -Dllvm=enabled \
    -Dplatforms= \
    -Dglx=disabled \
    -Dgbm=disabled \
    -Dgallium-va=disabled \
    -Dopengl=false \
    -Dgles1=disabled \
    -Dgles2=disabled \
    -Degl=disabled

echo "==> Building"
ninja -C "$MESA_DIR/build"

echo "==> Installing to $INSTALL_DIR"
rm -rf "$INSTALL_DIR"
ninja -C "$MESA_DIR/build" install

echo "==> Bundling Homebrew dylib dependencies"
LVP_DYLIB="$INSTALL_DIR/lib/libvulkan_lvp.dylib"

# Dylibs to bundle: source path -> bundled filename
declare -A DYLIBS=(
    ["/opt/homebrew/opt/llvm/lib/libLLVM.dylib"]="libLLVM.dylib"
    ["/opt/homebrew/opt/zstd/lib/libzstd.1.dylib"]="libzstd.1.dylib"
    ["/opt/homebrew/opt/spirv-tools/lib/libSPIRV-Tools.dylib"]="libSPIRV-Tools.dylib"
)

for src in "${!DYLIBS[@]}"; do
    dst="${DYLIBS[$src]}"
    echo "  Copying $src -> lib/$dst"
    cp "$src" "$INSTALL_DIR/lib/$dst"
    chmod 644 "$INSTALL_DIR/lib/$dst"
done

# Rewrite references in libvulkan_lvp.dylib
for src in "${!DYLIBS[@]}"; do
    dst="${DYLIBS[$src]}"
    echo "  Fixing reference: $src -> @loader_path/$dst"
    install_name_tool -change "$src" "@loader_path/$dst" "$LVP_DYLIB"
done

# Also fix cross-references between bundled dylibs (e.g., libLLVM may reference libzstd)
for lib in "$INSTALL_DIR/lib"/lib{LLVM,zstd.1,SPIRV-Tools}.dylib; do
    for src in "${!DYLIBS[@]}"; do
        dst="${DYLIBS[$src]}"
        # Only change if the reference exists (ignore errors)
        install_name_tool -change "$src" "@loader_path/$dst" "$lib" 2>/dev/null || true
    done
done

echo "==> Verifying dylib references"
otool -L "$LVP_DYLIB"

echo "==> Rewriting ICD JSON to use relative library_path"
ICD_JSON="$INSTALL_DIR/share/vulkan/icd.d/lvp_icd.aarch64.json"
python3 -c "
import json, sys
with open('$ICD_JSON') as f:
    data = json.load(f)
data['ICD']['library_path'] = '../../lib/libvulkan_lvp.dylib'
with open('$ICD_JSON', 'w') as f:
    json.dump(data, f, indent=4)
    f.write('\n')
"
echo "  ICD JSON:"
cat "$ICD_JSON"

echo "==> Creating tarball"
cd "$WORK_DIR"
# The tarball contains lib/ and share/ directories under a top-level folder
mv install "$ARTIFACT_NAME"
tar czf "$SCRIPT_DIR/$ARTIFACT_NAME.tar.gz" "$ARTIFACT_NAME"
mv "$ARTIFACT_NAME" install

echo ""
echo "==> Done! Artifact: $SCRIPT_DIR/$ARTIFACT_NAME.tar.gz"
echo ""
echo "To use:"
echo "  tar xzf $ARTIFACT_NAME.tar.gz"
echo "  export VK_DRIVER_FILES=\$(pwd)/$ARTIFACT_NAME/share/vulkan/icd.d/lvp_icd.aarch64.json"
echo "  vulkaninfo --summary"
