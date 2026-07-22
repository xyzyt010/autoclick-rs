#!/usr/bin/env bash
# Build static musl binaries for Linux x86_64 and aarch64.
# Prerequisites:
#   rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
#   For aarch64 cross: sudo apt install gcc-aarch64-linux-gnu  (or equivalent)
#
# Usage:
#   ./build.sh          # build both targets
#   ./build.sh x86      # build x86_64 only
#   ./build.sh arm      # build aarch64 only

set -euo pipefail

PROFILE="release"
OUT_DIR="target/dist"
mkdir -p "$OUT_DIR"

build_x86() {
    echo "==> Building x86_64-unknown-linux-musl (static)..."
    cargo build --release --target x86_64-unknown-linux-musl
    cp "target/x86_64-unknown-linux-musl/$PROFILE/autoclick-rs" "$OUT_DIR/autoclick-rs-x86_64"
    echo "    Output: $OUT_DIR/autoclick-rs-x86_64"
    file "$OUT_DIR/autoclick-rs-x86_64" 2>/dev/null || true
}

build_arm() {
    echo "==> Building aarch64-unknown-linux-musl (static)..."
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc
    cargo build --release --target aarch64-unknown-linux-musl
    cp "target/aarch64-unknown-linux-musl/$PROFILE/autoclick-rs" "$OUT_DIR/autoclick-rs-aarch64"
    echo "    Output: $OUT_DIR/autoclick-rs-aarch64"
    file "$OUT_DIR/autoclick-rs-aarch64" 2>/dev/null || true
}

case "${1:-all}" in
    x86|x86_64)
        build_x86
        ;;
    arm|aarch64)
        build_arm
        ;;
    all|"")
        build_x86
        build_arm
        ;;
    *)
        echo "Usage: $0 [x86|arm|all]"
        exit 1
        ;;
esac

echo ""
echo "==> Done. Binaries in $OUT_DIR/"
ls -lh "$OUT_DIR/"
