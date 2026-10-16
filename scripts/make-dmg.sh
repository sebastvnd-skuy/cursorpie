#!/usr/bin/env bash
# Builds a disk image to hand out: target/bundle/CursorPie-<version>-<arch>.dmg
# The image shows CursorPie.app next to an "Applications" shortcut, so installing is a drag.
#
# Usage:  scripts/make-dmg.sh                 (ad-hoc signature, this machine's architecture)
#         UNIVERSAL=1 scripts/make-dmg.sh     (Apple silicon + Intel in one image)
#         SIGN_ID="Developer ID Application: Name (TEAMID)" scripts/make-dmg.sh
#         SIGN_ID=... NOTARY_PROFILE=my-profile scripts/make-dmg.sh   (also notarizes and staples)
#         SKIP_BUNDLE=1 scripts/make-dmg.sh   (reuse the existing target/bundle/CursorPie.app)
#
# NOTARY_PROFILE is the name given to `xcrun notarytool store-credentials`. Notarizing needs a
# Developer ID signature, so it is refused without SIGN_ID.
# If `create-dmg` is installed (brew install create-dmg) the window gets a fixed size and icon
# positions; without it a plain image made with hdiutil is produced, which works the same.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ "$(uname -s)" != "Darwin" ]; then
    echo "make-dmg.sh runs on macOS only (it needs hdiutil and codesign)." >&2
    exit 1
fi
if [ -n "${NOTARY_PROFILE:-}" ] && [ -z "${SIGN_ID:-}" ]; then
    echo "NOTARY_PROFILE needs SIGN_ID: Apple only notarizes Developer ID signed apps." >&2
    exit 1
fi

APP="CursorPie"
VERSION=$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"(.*)".*/\1/')
if [ "${UNIVERSAL:-0}" = "1" ]; then ARCH="universal"; else ARCH=$(uname -m); fi

if [ "${SKIP_BUNDLE:-0}" != "1" ]; then
    scripts/bundle.sh
fi
APP_PATH="target/bundle/$APP.app"
[ -d "$APP_PATH" ] || { echo "Missing $APP_PATH: run scripts/bundle.sh first." >&2; exit 1; }

DMG="target/bundle/$APP-$VERSION-$ARCH.dmg"
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP_PATH" "$STAGE/"
rm -f "$DMG"

if command -v create-dmg >/dev/null 2>&1; then
    create-dmg \
        --volname "$APP" \
        --window-pos 200 120 \
        --window-size 540 340 \
        --icon-size 96 \
        --icon "$APP.app" 140 160 \
        --hide-extension "$APP.app" \
        --app-drop-link 400 160 \
        "$DMG" "$STAGE"
else
    ln -s /Applications "$STAGE/Applications"
    hdiutil create -volname "$APP" -srcfolder "$STAGE" -fs HFS+ -format UDZO -ov "$DMG"
fi

if [ -n "${SIGN_ID:-}" ]; then
    codesign --force --sign "$SIGN_ID" --timestamp "$DMG"
fi
if [ -n "${NOTARY_PROFILE:-}" ]; then
    xcrun notarytool submit "$DMG" --keychain-profile "$NOTARY_PROFILE" --wait
    xcrun stapler staple "$DMG"
fi

hdiutil verify "$DMG" >/dev/null
(cd "$(dirname "$DMG")" && shasum -a 256 "$(basename "$DMG")" > "$(basename "$DMG").sha256")
echo "Built $DMG"
echo "SHA-256: $(cut -d' ' -f1 "$DMG.sha256")"
if [ -z "${NOTARY_PROFILE:-}" ]; then
    echo "Not notarized: on another Mac, Gatekeeper asks to confirm the first start (right-click > Open)."
fi
