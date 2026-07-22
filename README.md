# autoclick-rs

A cross-platform automatic key presser with a native GUI, built in Rust + Slint.

Sends keystrokes to any window (GUI apps or terminals) at a configurable interval. Supports multiple independent instances (tabs), each targeting a different window.

## Downloads

Grab the latest binary from [Releases](https://github.com/xyzyt010/autoclick-rs/releases/latest):

| Platform | Asset | Notes |
|----------|-------|-------|
| Windows x86_64 | `autoclick-rs-windows-x86_64.exe` | Double-click to run |
| Linux x86_64 | `autoclick-rs-linux-x86_64` | Intel/AMD desktops |
| Linux aarch64 | `autoclick-rs-linux-aarch64` | Raspberry Pi, ARM VPS, etc. |

### Linux setup

```bash
# Make executable
chmod +x autoclick-rs-linux-x86_64   # or aarch64

# Run
./autoclick-rs-linux-x86_64
```

**X11** — works out of the box (uses XTest extension).

**Wayland** — needs access to `/dev/uinput`:
```bash
sudo usermod -aG input $USER
# Log out and back in, then run the app
```

## Features

- Native GUI (Slint) — no browser, no Electron
- Multiple independent instances via tabs
- Target any window by name (GUI apps + terminals)
- Configurable key, interval, and duration
- Background sending — target window doesn't need focus
- X11 (XTest) and Wayland (uinput) support on Linux
- PostMessage / SendInput / Console injection on Windows

## Build from source

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- Linux: `libfontconfig1-dev libx11-dev libxkbcommon-dev libwayland-dev`

### Windows

```bash
cd windows
cargo build --release
# Binary: windows/target/release/autoclick-rs.exe
```

### Linux

```bash
cd linux
cargo build --release
# Binary: linux/target/release/autoclick-rs
```

### Cross-compile Linux aarch64 (from x86_64)

```bash
sudo apt install gcc-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-gnu
cd linux
cargo build --release --target aarch64-unknown-linux-gnu
```

## Project structure

```
├── windows/          # Windows app (PostMessage, SendInput, Console)
│   ├── src/
│   └── ui/main.slint
├── linux/            # Linux app (X11 XTest, Wayland uinput)
│   ├── src/
│   └── ui/main.slint
└── .github/workflows/build.yml   # CI: builds all 3 targets
```

## CI/CD

Every push to `master` builds all 3 binaries. Pushing a version tag (`v*`) creates a GitHub Release with downloadable assets automatically.

```bash
git tag v1.1.0
git push origin v1.1.0
# → Release created at /releases/tag/v1.1.0
```

## License

MIT
