#!/usr/bin/env bash
# Builds target/bundle/cursorpie-<version>-linux-<arch>.tar.gz with the program, its desktop entry,
# its icon and an install.sh that copies them under a prefix (default /usr/local, or ~/.local with
# --user).
# Usage:  scripts/package-linux.sh
# Needs the development files of libx11, libxi, libxtst and libxrandr (see CONTRIBUTING.md).
# The square application icon is included in assets/app-icon.png; no image-conversion tool is needed.
# Build on an older distribution if the program must start on older ones (glibc is not backwards
# compatible).
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')
ARCH=$(uname -m)
NAME="cursorpie-$VERSION-linux-$ARCH"
OUT="target/bundle/$NAME"

cargo build --release
rm -rf "$OUT" "$OUT.tar.gz"
mkdir -p "$OUT"
cp target/release/cursorpie "$OUT/cursorpie"
cp packaging/linux/cursorpie.desktop "$OUT/cursorpie.desktop"
cp LICENSE README.md "$OUT/"

# Keep the white mark on the sage background square; do not use the horizontal wordmark here.
cp assets/app-icon.png "$OUT/cursorpie.png"

cat > "$OUT/install.sh" <<'INSTALL'
#!/usr/bin/env sh
# Installs CursorPie. Usage: ./install.sh [--user | --prefix DIR]
set -eu
here=$(cd "$(dirname "$0")" && pwd)
prefix=/usr/local
case "${1:-}" in
    --user) prefix="${XDG_DATA_HOME:-$HOME/.local/share}/.."; prefix=$(cd "$prefix" 2>/dev/null && pwd || echo "$HOME/.local") ;;
    --prefix) prefix=${2:?--prefix needs a directory} ;;
    "") ;;
    *) echo "usage: $0 [--user | --prefix DIR]" >&2; exit 2 ;;
esac
install -Dm755 "$here/cursorpie" "$prefix/bin/cursorpie"
install -Dm644 "$here/cursorpie.desktop" "$prefix/share/applications/cursorpie.desktop"
if [ -f "$here/cursorpie.png" ]; then
    install -Dm644 "$here/cursorpie.png" "$prefix/share/icons/hicolor/512x512/apps/cursorpie.png"
fi
echo "Installed under $prefix. Start CursorPie from your applications menu or run: cursorpie"
INSTALL
chmod +x "$OUT/install.sh"

tar -C target/bundle -czf "$OUT.tar.gz" "$NAME"
echo "Built $OUT.tar.gz"
