# devtop

A fast, lightweight, and interactive terminal system monitor and process explorer designed for developers, built in Rust with Ratatui.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)
![Ratatui](https://img.shields.io/badge/ratatui-0.30-purple.svg)

---

## Features

- **Overview Dashboard**:
  - High-resolution Braille time-series charts for global and per-core CPU usage.
  - Byte-accurate Memory telemetry (RAM, Swap, Reclaimable Cache) matching Linux kernel accounting.
  - Load average summary (1m, 5m, 15m).
  - Quick top active processes and storage preview.

- **Process Explorer**:
  - Live process table tracking PID, PPID, User, CPU%, Memory (Bytes & %), Runtime, and Commands.
  - **Process Tree Mode (`t`)**: Full DFS parent-child hierarchy visualization (`├─`, `└─`, `│ `).
  - **Process Detail Inspector (`Enter`)**: Deep drill-down view showing exe paths, current working directory (CWD), full command line, parent/child relationships, and bound ports.
  - Real-time search and filter (`/`).
  - Interactive multi-column sorting by CPU%, Memory, PID, Name, or User (`c`, `m`, `p`, `n`, `u`, `s`).

- **Listening Ports & Sockets**:
  - Native Linux `/proc/net/tcp` and `/proc/net/tcp6` socket scanner without invoking external binaries like `lsof` or `netstat`.
  - Automatic Inode-to-PID mapping with process metadata enrichment.
  - Direct process inspection and termination from open ports.

- **Storage & Network Telemetry**:
  - Mounted filesystem partitions, filesystem types, and space utilization.
  - Per-interface instantaneous Receive (Rx) and Transmit (Tx) speeds with rolling time-series graph.

- **Safe Signal Dispatcher (`k` / `x`)**:
  - Interactive signal selection modal defaulting to graceful termination (`15: SIGTERM`).
  - Support for `SIGKILL (9)`, `SIGINT (2)`, `SIGHUP (1)`, `SIGSTOP (19)`, and `SIGCONT (18)`.
  - Double confirmation and quick number selection (`1`-`6`) with immediate escape (`Esc`/`n`).

- **Optimized Engine**:
  - Split hot/slow telemetry polling loops: high-frequency sampling for CPU/Memory/Network/Processes, throttled 60s sampling for disks and users to prevent I/O blocking.
  - Robust terminal cleanup and panic hooks to preserve terminal state on exit.

---

## Installation & Setup

### Prerequisites
- [Rust & Cargo](https://www.rust-lang.org/tools/install) (version 1.80+ recommended).

### Build & Run
```bash
# Clone the repository
git clone https://github.com/Axyl1410/devtop.git
cd devtop

# Run in release mode
cargo run --release
```

### Running Tests
```bash
cargo test
```

---

## Keybindings

### Navigation & Views
| Key | Action |
| --- | --- |
| `Tab` / `Right` | Next Tab |
| `BackTab` / `Left` | Previous Tab |
| `1` | Switch to **Overview** |
| `2` | Switch to **Processes** |
| `3` | Switch to **Ports** |
| `4` | Switch to **Storage & Network** |
| `5` or `?` | Switch to **Help** |
| `Enter` | Inspect Process Detail (in Overview, Processes, Ports) |
| `Esc` / `Backspace` | Close Detail view / Clear search / Exit |

### Process & Table Controls
| Key | Action |
| --- | --- |
| `↑` / `↓` or `j` / `k` | Navigate items |
| `PageUp` / `PageDown` | Scroll page up / down |
| `t` | Toggle **Process Tree Mode** (Hierarchy view) |
| `s` | Cycle sort columns |
| `c` / `m` / `p` / `n` / `u` | Sort directly by CPU%, Memory, PID, Name, User |
| `/` | Search & filter table |
| `x` or `k` | Open Signal Dispatcher / Terminate Process modal |
| `+` / `-` | Adjust refresh rate (250ms - 5000ms) |
| `q` / `Ctrl+C` | Quit devtop |

---

## License

Licensed under the [MIT License](LICENSE).
