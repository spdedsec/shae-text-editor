#!/usr/bin/env bash
# Build a distributable release archive for Shae Text Editor.
set -euo pipefail

VERSION="${1:-$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')}"
ARCH="$(uname -m)"
NAME="shae-text-editor"
DIST="dist"
PKG="$NAME-linux-$ARCH"

say() { printf '\033[1;33m==>\033[0m %s\n' "$*"; }

say "Building release binary (v$VERSION)"
cargo build --release

say "Assembling $PKG"
rm -rf "$DIST/$PKG"
mkdir -p "$DIST/$PKG/assets"

cp "target/release/$NAME"      "$DIST/$PKG/"
cp README.md GUIDE.md LICENSE  "$DIST/$PKG/"
cp install.sh                  "$DIST/$PKG/"
cp assets/icon.svg             "$DIST/$PKG/assets/"
cp assets/icon-*.png           "$DIST/$PKG/assets/"
cp assets/shae-text-editor.desktop "$DIST/$PKG/assets/"

chmod +x "$DIST/$PKG/$NAME" "$DIST/$PKG/install.sh"

say "Creating archive"
tar -C "$DIST" -czf "$DIST/$PKG.tar.gz" "$PKG"

# Checksums
( cd "$DIST" && sha256sum "$PKG.tar.gz" > "$PKG.tar.gz.sha256" )

say "Done:"
ls -lh "$DIST/$PKG.tar.gz" "$DIST/$PKG.tar.gz.sha256"
echo
echo "Install with:"
echo "  tar -xzf $DIST/$PKG.tar.gz"
echo "  cd $PKG && ./install.sh"