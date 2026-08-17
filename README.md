# ⚡ devtop

A modern, fast, and interactive developer system monitor & process manager in your terminal, built with [Rust](https://www.rust-lang.org/), [Ratatui](https://ratatui.rs/), [Crossterm](https://github.com/crossterm-rs/crossterm), and [Sysinfo](https://github.com/GuillaumeGomez/sysinfo).

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)
![Ratatui](https://img.shields.io/badge/ratatui-0.30-purple.svg)

---

## ✨ Features

- 💻 **System Overview**: Real-time CPU, RAM, Swap gauges, Sparkline history graphs, Hostname, Kernel, Uptime, and load summary.
- 📋 **Process Manager**:
  - Live process table with PID, CPU %, Memory usage (MB & %), Status, and Commands.
  - Interactive multi-column sorting (`CPU%`, `Memory`, `PID`, `Name`).
  - Search & filter processes on the fly (`/`).
  - Safe process termination with double-confirmation dialog (`x` / `k`).
- 💾 **Storage & Network Monitor**:
  - Mounted disk partitions, filesystem types, used/free space.
  - Real-time network Download & Upload speed trackers with live sparklines.
- 🎨 **Sleek Terminal UI**: Fully styled with rounded borders, responsive layouts, color status indicators, and keyboard shortcuts.
- 🛡️ **Robust Terminal Recovery**: Clean raw mode teardown and custom panic hook to protect your terminal state.

---

## 🚀 Installation & Setup

### Prerequisites
- [Rust & Cargo](https://www.rust-lang.org/tools/install) installed (version 1.80+ recommended).

### Clone and Run
```bash
# Clone the repository
git clone https://github.com/Axyl1410/devtop.git
cd devtop

# Build & Run
cargo run --release
```

---

## ⌨️ Keybindings

| Key | Action |
| --- | --- |
| `Tab` / `Right` | Next Tab |
| `BackTab` / `Left` | Previous Tab |
| `1` | Switch to **Overview** |
| `2` | Switch to **Processes** |
| `3` | Switch to **Storage & Network** |
| `4` or `?` | Switch to **Help** |
| `↑` / `↓` or `j` / `k` | Select Process |
| `s` | Cycle sort columns |
| `c` | Sort by CPU % |
| `m` | Sort by Memory |
| `p` | Sort by PID |
| `n` | Sort by Name |
| `/` | Search / Filter processes |
| `x` or `k` | Terminate selected process (Confirm dialog) |
| `+` / `-` | Adjust refresh rate (250ms - 5000ms) |
| `q` / `Esc` / `Ctrl+C` | Quit devtop |

---

## 📄 License

Licensed under the [MIT License](LICENSE).
