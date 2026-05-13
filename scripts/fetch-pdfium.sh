#!/usr/bin/env bash
set -euo pipefail

ARCH=$(uname -m)
case "$ARCH" in
  arm64)   PKG="pdfium-mac-arm64.tgz" ;;
  x86_64)  PKG="pdfium-mac-x64.tgz"   ;;
  *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT_DIR/src-tauri/lib"
mkdir -p "$OUT_DIR"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/$PKG"
echo "Downloading $URL"
curl -fL "$URL" -o "$TMP/pdfium.tgz"
tar -xzf "$TMP/pdfium.tgz" -C "$TMP"

cp "$TMP/lib/libpdfium.dylib" "$OUT_DIR/libpdfium.dylib"
echo "PDFium installed at $OUT_DIR/libpdfium.dylib"
