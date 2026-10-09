#!/usr/bin/env bash
# Builds target/bundle/CursorPie.app (menu-bar only app, no Dock icon).
# Usage:  scripts/bundle.sh                 (ad-hoc signature)
#         SIGN_ID="Developer ID Application: Name (TEAMID)" scripts/bundle.sh
set -euo pipefail
cd "$(dirname "$0")/.."

APP="CursorPie"
BUNDLE_ID="io.github.cursorpie"   # keep in sync with src/about.rs
VERSION=$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')

cargo build --release
OUT="target/bundle/$APP.app"
rm -rf "$OUT"
mkdir -p "$OUT/Contents/MacOS" "$OUT/Contents/Resources"
cp target/release/cursorpie "$OUT/Contents/MacOS/$APP"

cat > "$OUT/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>$APP</string>
  <key>CFBundleDisplayName</key><string>$APP</string>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundleExecutable</key><string>$APP</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST

codesign --force --deep --sign "${SIGN_ID:--}" "$OUT"
echo "Built $OUT"
echo "Note: with the ad-hoc signature macOS may ask you to re-grant Accessibility after every rebuild."
