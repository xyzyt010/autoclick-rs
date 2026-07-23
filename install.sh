#!/usr/bin/env bash
# AutoClick-RS installer for Linux
# Installs binary + .desktop entry so the app appears in application search.
set -e

REPO="xyzyt010/autoclick-rs"
INSTALL_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/256x256/apps"
DESKTOP_FILE="${APP_DIR}/autoclick-rs.desktop"
BINARY="${INSTALL_DIR}/autoclick-rs"

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64) ASSET="autoclick-rs-linux-x86_64" ;;
    aarch64|arm64) ASSET="autoclick-rs-linux-aarch64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "==> Downloading AutoClick-RS (${ARCH})..."
mkdir -p "$INSTALL_DIR"
curl -fSL -o "$BINARY" \
    "https://github.com/${REPO}/releases/latest/download/${ASSET}"
chmod +x "$BINARY"

# Verify binary was downloaded
if [ ! -x "$BINARY" ]; then
    echo "ERROR: Download failed or binary not executable." >&2
    exit 1
fi
echo "    Binary: $BINARY"

# Remove old .desktop file to avoid stale cache
echo "==> Installing desktop entry..."
mkdir -p "$APP_DIR"
rm -f "$DESKTOP_FILE"

# Write .desktop file with absolute paths (no heredoc to avoid pipe/CRLF issues)
printf '[Desktop Entry]\n' > "$DESKTOP_FILE"
printf 'Name=AutoClick-RS\n' >> "$DESKTOP_FILE"
printf 'GenericName=Automatic Key Presser\n' >> "$DESKTOP_FILE"
printf 'Comment=Cross-platform automatic key presser with native GUI\n' >> "$DESKTOP_FILE"
printf 'Exec=%s\n' "$BINARY" >> "$DESKTOP_FILE"
printf 'Icon=%s/autoclick-rs.png\n' "$ICON_DIR" >> "$DESKTOP_FILE"
printf 'Terminal=false\n' >> "$DESKTOP_FILE"
printf 'Type=Application\n' >> "$DESKTOP_FILE"
printf 'Categories=Utility;Accessibility;\n' >> "$DESKTOP_FILE"
printf 'Keywords=autoclick;keypress;automation;macro;\n' >> "$DESKTOP_FILE"
printf 'StartupNotify=true\n' >> "$DESKTOP_FILE"
chmod +x "$DESKTOP_FILE"

# Mark as trusted (GNOME requires this for .desktop files in ~/.local)
if command -v gio &>/dev/null; then
    gio set "$DESKTOP_FILE" metadata::trusted true 2>/dev/null || true
fi

# Install icon
echo "==> Installing icon..."
mkdir -p "$ICON_DIR"
curl -fsSL -o "${ICON_DIR}/autoclick-rs.png" \
    "https://raw.githubusercontent.com/${REPO}/master/assets/logo.png" 2>/dev/null || true

# Refresh desktop database
if command -v update-desktop-database &>/dev/null; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

# Touch to force filesystem timestamp update (helps some DEs pick up changes)
touch "$DESKTOP_FILE"

echo ""
echo "==> AutoClick-RS installed successfully!"
echo "    Binary:  $BINARY"
echo "    Desktop: $DESKTOP_FILE"
echo ""
echo "    Run from terminal: $BINARY"
echo "    Or search 'AutoClick-RS' in your application launcher."
echo ""
echo "    If it still doesn't launch from the menu, try logging out and back in."
echo ""
