#!/usr/bin/env bash
set -euo pipefail

APP_NAME="Cococa Clip"
DMG_NAME="Cococa-Clip"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
STAGE_DIR="$DIST_DIR/dmg-stage"
BUNDLE_DIR="$ROOT_DIR/target/release/bundle/osx"
APP_PATH="$BUNDLE_DIR/$APP_NAME.app"

mkdir -p "$DIST_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found"
  exit 1
fi

if ! cargo bundle --help >/dev/null 2>&1; then
  echo "cargo-bundle not installed, installing..."
  cargo install cargo-bundle
fi

echo "Building .app with cargo bundle..."
cd "$ROOT_DIR"
cargo bundle --release

if [ ! -d "$APP_PATH" ]; then
  echo "error: app bundle not found at $APP_PATH"
  exit 1
fi

echo "Preparing DMG staging folder..."
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"
cp -R "$APP_PATH" "$STAGE_DIR/"
ln -s /Applications "$STAGE_DIR/Applications"

echo "Creating DMG..."
rm -f "$DIST_DIR/$DMG_NAME.dmg"
hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$STAGE_DIR" \
  -ov \
  -format UDZO \
  "$DIST_DIR/$DMG_NAME.dmg"

echo "Done: $DIST_DIR/$DMG_NAME.dmg"
