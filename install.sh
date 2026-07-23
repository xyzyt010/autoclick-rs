#!/usr/bin/env bash
# AutoClick-RS installer for Linux
# Installs binary + .desktop entry so the app appears in application search.
set -e

REPO="xyzyt010/autoclick-rs"
INSTALL_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/256x256/apps"

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64) ASSET="autoclick-rs-linux-x86_64" ;;
    aarch64|arm64) ASSET="autoclick-rs-linux-aarch64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "==> Downloading AutoClick-RS (${ARCH})..."
mkdir -p "$INSTALL_DIR"
curl -L -o "${INSTALL_DIR}/autoclick-rs" \
    "https://github.com/${REPO}/releases/latest/download/${ASSET}"
chmod +x "${INSTALL_DIR}/autoclick-rs"

# Install .desktop entry
echo "==> Installing desktop entry..."
mkdir -p "$APP_DIR"
cat > "${APP_DIR}/autoclick-rs.desktop" << 'EOF'
[Desktop Entry]
Name=AutoClick-RS
GenericName=Automatic Key Presser
Comment=Cross-platform automatic key presser with native GUI
Exec=autoclick-rs
Icon=autoclick-rs
Terminal=false
Type=Application
Categories=Utility;Accessibility;
Keywords=autoclick;keypress;automation;macro;
StartupNotify=true
EOF

# Install icon
echo "==> Installing icon..."
mkdir -p "$ICON_DIR"
curl -sL -o "${ICON_DIR}/autoclick-rs.png" \
    "https://raw.githubusercontent.com/${REPO}/master/assets/logo.png" 2>/dev/null || true

# Update desktop database if available
if command -v update-desktop-database &>/dev/null; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

# Ensure ~/.local/bin is in PATH
if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    echo ""
    echo "NOTE: ${INSTALL_DIR} is not in your PATH."
    echo "Add this to your ~/.bashrc or ~/.zshrc:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

echo ""
echo "==> AutoClick-RS installed successfully!"
echo "    Run:    autoclick-rs"
echo "    Search: Type 'AutoClick-RS' in your application launcher"
echo ""
