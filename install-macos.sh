#!/usr/bin/env bash
# AutoClick-RS installer for macOS (Apple Silicon)
# Installs binary to ~/.local/bin and removes quarantine flag.
set -e

REPO="xyzyt010/autoclick-rs"
INSTALL_DIR="${HOME}/.local/bin"
BINARY="${INSTALL_DIR}/autoclick-rs"
ASSET="autoclick-rs-macos-aarch64"

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    arm64|aarch64) ;; # Apple Silicon — supported
    x86_64) echo "ERROR: Only Apple Silicon (M1/M2/M3/M4) is currently supported."; exit 1 ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "==> Downloading AutoClick-RS (macOS ${ARCH})..."
mkdir -p "$INSTALL_DIR"
curl -fSL -o "$BINARY" \
    "https://github.com/${REPO}/releases/latest/download/${ASSET}"
chmod +x "$BINARY"

# Remove macOS quarantine attribute (Gatekeeper)
xattr -d com.apple.quarantine "$BINARY" 2>/dev/null || true

# Verify binary was downloaded
if [ ! -x "$BINARY" ]; then
    echo "ERROR: Download failed or binary not executable." >&2
    exit 1
fi

# Ensure ~/.local/bin is in PATH (add to .zshrc if missing)
SHELL_RC="${HOME}/.zshrc"
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    if [ -f "$SHELL_RC" ] && ! grep -q "$INSTALL_DIR" "$SHELL_RC"; then
        echo "" >> "$SHELL_RC"
        echo "# Added by AutoClick-RS installer" >> "$SHELL_RC"
        echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$SHELL_RC"
        echo "    Added ~/.local/bin to PATH in $SHELL_RC"
    fi
fi

echo ""
echo "==> AutoClick-RS installed successfully!"
echo "    Binary: $BINARY"
echo ""
echo "    Run: $BINARY"
echo "    Or:  autoclick-rs  (if ~/.local/bin is in PATH)"
echo ""
echo "    IMPORTANT: On first run, grant Accessibility permission:"
echo "    System Settings → Privacy & Security → Accessibility → enable autoclick-rs"
echo ""
