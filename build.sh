#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORK_DIR="$SCRIPT_DIR/_work"
INSTALL_DIR="$WORK_DIR/install"
MESA_DIR="$WORK_DIR/mesa"
VENV_DIR="$WORK_DIR/.venv"
ARTIFACT_NAME="lavapipe-macos-arm64"

echo "==> Installing Homebrew dependencies"
brew update
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

echo "==> Rewriting ICD JSON to use relative library_path"
ICD_JSON="$INSTALL_DIR/share/vulkan/icd.d/lvp_icd.aarch64.json"
python3 -c "
import json
with open('$ICD_JSON') as f:
    data = json.load(f)
data['ICD']['library_path'] = './libvulkan_lvp.dylib'
with open('$ICD_JSON', 'w') as f:
    json.dump(data, f, indent=4)
    f.write('\n')
"

echo "==> Creating tarball"
STAGING_DIR="$WORK_DIR/$ARTIFACT_NAME"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR"
cp "$INSTALL_DIR/lib/libvulkan_lvp.dylib" "$STAGING_DIR/"
cp "$ICD_JSON" "$STAGING_DIR/"
echo "  Contents:"
ls -la "$STAGING_DIR"
echo "  ICD JSON:"
cat "$STAGING_DIR/lvp_icd.aarch64.json"
cd "$WORK_DIR"
tar czf "$SCRIPT_DIR/$ARTIFACT_NAME.tar.gz" "$ARTIFACT_NAME"

echo ""
echo "==> Done! Artifact: $SCRIPT_DIR/$ARTIFACT_NAME.tar.gz"
echo ""
echo "To use:"
echo "  tar xzf $ARTIFACT_NAME.tar.gz"
echo "  export VK_DRIVER_FILES=\$(pwd)/$ARTIFACT_NAME/lvp_icd.aarch64.json"
echo "  vulkaninfo --summary"
