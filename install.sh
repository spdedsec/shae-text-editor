#!/usr/bin/env bash
# Shae Text Editor — installer for Linux
# Installs the binary to ~/.local/bin and registers a desktop entry.
set -euo pipefail

BIN_NAME="shae-text-editor"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

say()  { printf '\033[1;33m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m  ✓\033[0m %s\n' "$*"; }
warn() { printf '\033[1;31m  !\033[0m %s\n' "$*"; }
die()  { warn "$*"; exit 1; }

# ---- Locate the binary -------------------------------------------------------
BIN_SRC=""
for candidate in \
  "$SCRIPT_DIR/$BIN_NAME" \
  "$SCRIPT_DIR/target/release/$BIN_NAME" \
  "$SCRIPT_DIR/../target/release/$BIN_NAME"; do
  if [ -x "$candidate" ]; then BIN_SRC="$candidate"; break; fi
done
[ -n "$BIN_SRC" ] || die "Could not find the '$BIN_NAME' binary next to this script."

# ---- Check runtime dependencies ---------------------------------------------
say "Checking runtime libraries"
missing=()
if command -v ldd >/dev/null 2>&1; then
  while IFS= read -r line; do
    case "$line" in
      *"not found"*) missing+=("$(echo "$line" | awk '{print $1}')");;
    esac
  done < <(ldd "$BIN_SRC" 2>/dev/null || true)
fi
if [ ${#missing[@]} -gt 0 ]; then
  warn "Missing shared libraries: ${missing[*]}"
  echo "   Arch/Omarchy:  sudo pacman -S libgit2 openssl"
  echo "   Debian/Ubuntu: sudo apt install libgit2-1.7 libssl3"
  echo "   Fedora:        sudo dnf install libgit2 openssl-libs"
  die "Install the libraries above, then re-run this installer."
fi
ok "All runtime libraries present"

# ---- Install binary ----------------------------------------------------------
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
install -m 0755 "$BIN_SRC" "$BIN_DIR/$BIN_NAME"
ok "Installed binary -> $BIN_DIR/$BIN_NAME"

# ---- Install desktop entry + icon -------------------------------------------
DATA_DIR="$HOME/.local/share"
ICON_DIR="$DATA_DIR/icons/hicolor"
APPS_DIR="$DATA_DIR/applications"
mkdir -p "$APPS_DIR"

if [ -f "$SCRIPT_DIR/assets/shae-text-editor.desktop" ]; then
  install -m 0644 "$SCRIPT_DIR/assets/shae-text-editor.desktop" "$APPS_DIR/shae-text-editor.desktop"
  ok "Installed desktop entry"
fi

installed_icon=0
for size in 512 256 128 64 32; do
  src="$SCRIPT_DIR/assets/icon-$size.png"
  if [ -f "$src" ]; then
    dest="$ICON_DIR/${size}x${size}/apps"
    mkdir -p "$dest"
    install -m 0644 "$src" "$dest/shae-text-editor.png"
    installed_icon=1
  fi
done
[ "$installed_icon" = 1 ] && ok "Installed application icons"

# ---- Refresh caches ----------------------------------------------------------
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS_DIR" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$ICON_DIR" 2>/dev/null || true

# ---- PATH hint ---------------------------------------------------------------
case ":$PATH:" in
  *":$BIN_DIR:"*) : ;;
  *) warn "$BIN_DIR is not on your PATH. Add this to your shell profile:"
     echo '     export PATH="$HOME/.local/bin:$PATH"' ;;
esac

echo
ok "Shae Text Editor installed."
echo "   Run it with:  shae-text-editor"
echo "   Or launch \"Shae Text Editor\" from your app menu."
