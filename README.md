<p align="center">
  <img src="assets/logo.png" width="128" height="128" alt="AutoClick-RS Logo">
</p>

<h1 align="center">AutoClick-RS</h1>

<p align="center">
  A cross-platform automatic key presser with a native GUI, built in Rust + Slint.<br>
  Sends keystrokes to any window (GUI apps or terminals) at a configurable interval.
</p>

---

## Quick Install (copy, paste, run)

Find your platform below, copy the command, paste it into your terminal, and press Enter.

### Linux (all distros — Ubuntu, Fedora, Arch, Debian, etc.)

**One-liner install** (installs to `~/.local/bin` + registers in app search):

```bash
curl -fsSL https://raw.githubusercontent.com/xyzyt010/autoclick-rs/main/install.sh | bash
```

After install, type **AutoClick-RS** in your application launcher (GNOME Activities, KDE menu, rofi, dmenu, etc.) — it will appear.

Or run directly without installing:

```bash
# x86_64 (Intel/AMD)
curl -L -o autoclick-rs https://github.com/xyzyt010/autoclick-rs/releases/latest/download/autoclick-rs-linux-x86_64 && chmod +x autoclick-rs && ./autoclick-rs

# aarch64 (ARM64 — Raspberry Pi, ARM VPS)
curl -L -o autoclick-rs https://github.com/xyzyt010/autoclick-rs/releases/latest/download/autoclick-rs-linux-aarch64 && chmod +x autoclick-rs && ./autoclick-rs
```

### macOS (Apple Silicon — M1/M2/M3/M4)

```bash
curl -L -o autoclick-rs https://github.com/xyzyt010/autoclick-rs/releases/latest/download/autoclick-rs-macos-aarch64 && chmod +x autoclick-rs && xattr -d com.apple.quarantine autoclick-rs 2>/dev/null; ./autoclick-rs
```

> **macOS first-run**: Grant Accessibility permission:
> **System Settings → Privacy & Security → Accessibility → enable `autoclick-rs`**
> Then run the command again.

### Windows x86_64 (PowerShell)

```powershell
curl.exe -L -o autoclick-rs.exe https://github.com/xyzyt010/autoclick-rs/releases/latest/download/autoclick-rs-windows-x86_64.exe; .\autoclick-rs.exe
```

Or download `autoclick-rs-windows-x86_64.exe` from [Releases](https://github.com/xyzyt010/autoclick-rs/releases/latest) and double-click it.

---

## App Discovery

| Platform | How to find it |
|----------|----------------|
| **Linux** | Run the install script above → type `AutoClick-RS` in your app launcher (GNOME, KDE, XFCE, rofi, dmenu, etc.) |
| **Windows** | Search `autoclick-rs` in Start Menu after placing the exe in a PATH folder, or pin it to Start |
| **macOS** | Run from terminal, or move to `/Applications` and Spotlight will index it |

---

## How it works

1. Launch the app → GUI opens
2. Pick **Terminal** or **Window** mode
3. Click **Refresh** → see all open windows (terminals, browsers, editors, etc.)
4. **Type to search** — filter windows instantly by typing in the search box
5. Select your target window
6. Choose a key, set interval (seconds) and optional duration (minutes)
7. Click **Start** → keys are injected into that window automatically

The app correctly identifies and lists your open windows by name — not just the display server or window manager.

---

## Platform notes

### Linux (X11)

Works out of the box on any X11 desktop (XFCE, GNOME, KDE, i3, etc.).
Uses XTest extension with XSendEvent fallback — no extra setup needed.

### Linux (Wayland)

Uses a virtual keyboard via `/dev/uinput`. One-time setup:

```bash
sudo modprobe uinput
sudo usermod -aG input $USER
# Log out and back in
```

### macOS

Uses CGEvent API. Requires Accessibility permission (granted once).
Keys are sent to the selected target app — the app is activated before each key press.

### Windows

Uses PostMessage/SendInput. Works with both GUI apps and terminal windows (cmd, PowerShell, Windows Terminal).

---

## All release assets

| Platform | File | Size |
|----------|------|------|
| Windows x86_64 | `autoclick-rs-windows-x86_64.exe` | ~9 MB |
| Linux x86_64 | `autoclick-rs-linux-x86_64` | ~12 MB |
| Linux aarch64 | `autoclick-rs-linux-aarch64` | ~11 MB |
| macOS aarch64 | `autoclick-rs-macos-aarch64` | ~8 MB |

All binaries are self-contained executables — no installer, no dependencies to install.

---

## Build from source

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- Linux: `sudo apt install libfontconfig1-dev libxkbcommon-dev libwayland-dev`

### Windows

```bash
cd windows
cargo build --release
# → windows/target/release/autoclick-rs.exe
```

### Linux

```bash
cd linux
cargo build --release
# → linux/target/release/autoclick-rs
```

### macOS

```bash
cd macos
cargo build --release
# → macos/target/release/autoclick-rs
```

---

## Project structure

```
├── assets/           # Logo + .desktop file
├── install.sh        # Linux one-liner installer
├── windows/          # Windows app (PostMessage, SendInput)
│   ├── src/
│   └── ui/main.slint
├── linux/            # Linux app (X11 XTest/XSendEvent, Wayland uinput)
│   ├── src/
│   └── ui/main.slint
├── macos/            # macOS app (CGEvent + app activation)
│   ├── src/
│   └── ui/main.slint
└── .github/workflows/build.yml   # CI: builds all 4 targets
```

## CI/CD

Pushing a version tag (`v*`) creates a GitHub Release with all binaries automatically.

```bash
git tag v1.3.0
git push origin v1.3.0
# → Release at /releases/tag/v1.3.0
```

## License

MIT
