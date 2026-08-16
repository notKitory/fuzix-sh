# fuzix (CLI)

[English](./README.md) | [Русский](./README_RU.md)

`fuzix` is a modern, native CLI developer tool and SDK for developing, compiling, and testing applications for **FUZIX OS**.

🚀 **100% Native & No Docker:** Runs natively on your host machine (macOS Apple Silicon / Intel, Linux, Windows) without Docker overhead.  
🎮 **Full EmulatorKit Power:** Supports dozens of boards and CPU targets (Motorola 68000, Z80, Z180, 6809, 6502, 8080/8085, and more).  
⚡ **Blazing Fast:** Native compilation toolchains and pure PTY-based emulator automation.

---

## 📦 Installation

### From Source (Cargo)
```bash
cargo build --release
# Binary will be built at target/release/fuzix
```

---

## 🛠 Quickstart

### 1. Initialize a new project
```bash
fuzix init --name my-app --cpu 68000 --emulator v68
```
This generates `fuzix.toml` and a template `hello.c`.

### 2. Build C source
```bash
fuzix build hello.c
```

### 3. Automated Testing (Build + Disk Inject + Run)
```bash
fuzix test hello.c arg1 arg2
```

### 4. Interactive Console (FUZIX Shell)
```bash
fuzix shell
```
*Press `Ctrl-]` or type `shutdown` to exit the emulator.*

---

## 📋 Commands

| Command | Description |
| --- | --- |
| `fuzix init` | Initializes a new FUZIX project and creates `fuzix.toml`. |
| `fuzix build [source.c]` | Compiles a C source file or invokes `make` with native FUZIX toolchain. |
| `fuzix test [source.c]` | Automatically compiles, injects binary into disk image, and runs test. |
| `fuzix run <cmd> [args...]` | Runs a command inside the FUZIX emulator and captures output. |
| `fuzix shell` | Opens an interactive PTY terminal session with the emulator. |
| `fuzix disk cp <host> <fuzix>` | Injects a host file into the FUZIX root disk image (via `ucp`). |
| `fuzix disk ls [path]` | Lists files on the FUZIX disk image (default: `/bin`). |
| `fuzix emulators` | Lists supported emulators and boards from EmulatorKit. |

---

## ⚙️ Configuration (`fuzix.toml`)

Project configuration is stored in `fuzix.toml`:

```toml
[project]
name = "my-app"
version = "0.1.0"
source = "hello.c"

[target]
cpu = "68000"          # 68000, z80, 8080, 6809, 6502
emulator = "v68"        # v68, tiny68k, rc2014, cpmsim, swt6809, rcbus-6502
timeout = 45

[disk]
boot_image = ".fuzix/images/boot.dsk"
root_image = ".fuzix/images/hd-fuzix.dsk"

[toolchain]
repo = "notKitory/fuzix-sh"
release = "latest"
```

---

## 🕹 Supported Emulators (EmulatorKit & Virtual68)

Run `fuzix emulators` to view all available systems:

*   **`v68`** — Motorola 68000 with software MMU and IDE disk support *(Recommended for 68k)*.
*   **`tiny68k`** / **`mini68k`** — 68000 Single Board Computers.
*   **`rc2014`** / **`rcbus-z80`** — Z80 modular bus retrocomputers.
*   **`rcbus-z180`** — Z180 high-speed enhanced system.
*   **`cpmsim`** — 8080 / Z80 CP/M & FUZIX simulator (z80pack).
*   **`swt6809`** — Motorola 6809 system.
*   **`rcbus-6502`** — MOS 6502 system.
*   **`altair8080`** — MITS Altair 8800 S-100 system.

---

## 🤝 Contributing

Contributions and Pull Requests are welcome on [GitHub](https://github.com/notKitory/fuzix-sh)!
