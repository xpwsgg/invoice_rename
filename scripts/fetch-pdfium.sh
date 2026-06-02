#!/usr/bin/env bash
set -euo pipefail

ARCH="${1:-$(uname -m)}"

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT_DIR/src-tauri/lib"
mkdir -p "$OUT_DIR"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

download_pdfium() {
  local arch="$1" pkg
  case "$arch" in
    arm64)   pkg="pdfium-mac-arm64.tgz" ;;
    x86_64)  pkg="pdfium-mac-x64.tgz"   ;;
    *) echo "Unsupported arch: $arch" >&2; exit 1 ;;
  esac
  local url="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/$pkg"
  # 日志必须走 stderr：本函数的 stdout 是“返回值”（库路径），会被 $(...) 捕获。
  # 若日志混入 stdout，调用方拿到的路径会带上这行文本，导致 lipo/cp 拿到非法文件名。
  echo "Downloading $url" >&2
  curl -fL "$url" -o "$TMP/$arch.tgz"
  mkdir -p "$TMP/$arch"
  tar -xzf "$TMP/$arch.tgz" -C "$TMP/$arch"
  echo "$TMP/$arch/lib/libpdfium.dylib"
}

if [ "$ARCH" = "universal" ]; then
  arm64_lib=$(download_pdfium arm64)
  x86_64_lib=$(download_pdfium x86_64)
  echo "Creating universal binary with lipo..."
  lipo -create "$arm64_lib" "$x86_64_lib" -output "$OUT_DIR/libpdfium.dylib"
else
  lib=$(download_pdfium "$ARCH")
  cp "$lib" "$OUT_DIR/libpdfium.dylib"
fi

echo "PDFium installed at $OUT_DIR/libpdfium.dylib"
