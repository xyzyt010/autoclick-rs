#!/bin/bash
# build-app.sh — Assemble AutoClick-RS.app from the release binary
# Usage: ./build-app.sh [path/to/autoclick-rs]
#
# Produces AutoClick-RS.app in the current directory.
# Requires: sips, iconutil (both ship with macOS).

set -euo pipefail

BIN="${1:-target/release/autoclick-rs}"
APP="AutoClick-RS.app"
CONTENTS="$APP/Contents"
MACOS_DIR="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"

if [ ! -f "$BIN" ]; then
    echo "Binary not found at $BIN — run 'cargo build --release' first."
    exit 1
fi

# Clean previous output
rm -rf "$APP"

# Create directory structure
mkdir -p "$MACOS_DIR" "$RESOURCES"

# --- Info.plist ---------------------------------------------------------------
cat > "$CONTENTS/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>AutoClick-RS</string>
    <key>CFBundleDisplayName</key>
    <string>AutoClick-RS</string>
    <key>CFBundleIdentifier</key>
    <string>com.xyzyt010.autoclick-rs</string>
    <key>CFBundleVersion</key>
    <string>1.6.2</string>
    <key>CFBundleShortVersionString</key>
    <string>1.6.2</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>autoclick-rs</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright (c) 2026 xyzyt010. MIT License.</string>
</dict>
</plist>
PLIST

# --- Icon --------------------------------------------------------------------
LOGO=""
if [ -f "../assets/logo.png" ]; then
    LOGO="../assets/logo.png"
elif [ -f "assets/logo.png" ]; then
    LOGO="assets/logo.png"
fi
if [ -n "$LOGO" ]; then
    ICONSET="AppIcon.iconset"
    rm -rf "$ICONSET"
    mkdir -p "$ICONSET"

    for SIZE in 16 32 64 128 256 512; do
        sips -z $SIZE $SIZE "$LOGO" --out "$ICONSET/icon_${SIZE}x${SIZE}.png" >/dev/null 2>&1
        DOUBLE=$((SIZE * 2))
        sips -z $DOUBLE $DOUBLE "$LOGO" --out "$ICONSET/icon_${SIZE}x${SIZE}@2x.png" >/dev/null 2>&1
    done

    iconutil -c icns "$ICONSET" -o "$RESOURCES/AppIcon.icns"
    rm -rf "$ICONSET"
    echo "Icon generated: $RESOURCES/AppIcon.icns"
else
    echo "Warning: logo.png not found — no icon embedded."
fi

# --- Binary -------------------------------------------------------------------
cp "$BIN" "$MACOS_DIR/autoclick-rs"
chmod +x "$MACOS_DIR/autoclick-rs"

echo ""
echo "Built: $APP"
echo "  Size: $(du -sh "$APP" | cut -f1)"
echo "  Contents:"
ls -la "$MACOS_DIR/"
ls -la "$RESOURCES/" 2>/dev/null || true
