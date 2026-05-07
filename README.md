# fuzix-sh

[English](./README.md) | [Русский](./README_RU.md)

`fuzix-sh` is a small shell wrapper for compiling and running C programs for FUZIX through z80pack.

## Requirements

- Docker
- `curl`
- `tar`

## Usage

```bash
./fuzix.sh compile <source.c>
./fuzix.sh image
./fuzix.sh run [-v] [arg...]
./fuzix.sh shell
./fuzix.sh test [-v] <source.c> [arg...]
```

On the first run, the script downloads binaries into `.fuzix-sh/prebuilt/<arch>` and builds a Docker runtime image. Later runs reuse this runtime.

## Commands

| Command | Description |
| --- | --- |
| `compile <source.c>` | Compiles a C source file into `.fuzix-sh/bin/<source-name>` and makes it the current program. |
| `image` | Copies the current compiled program into `/bin` inside the FUZIX root disk. |
| `run [-v] [arg...]` | Boots FUZIX in z80pack, runs the current program, prints the program output, and shuts the emulator down. |
| `shell` | Opens an interactive FUZIX shell. `Ctrl-]` force-shuts down the emulator. |
| `test [-v] <source.c> [arg...]` | Runs `compile`, `image`, and `run` in sequence. |

Use `-v` to show emulator output instead of only the program output.

Program arguments are passed directly after the command:

```bash
./fuzix.sh test hello.c arg1 arg2
```

## State Directory

By default, all generated files are stored in `.fuzix-sh`:

```text
.fuzix-sh
├── bin
├── build
├── images
│   ├── boot.dsk
│   └── hd-fuzix.dsk
└── prebuilt
    └── <arch>
```

`hd-fuzix.dsk` is a single mutable root disk. Every `image` writes the current program into the same disk image.

## Development

If you have ideas, suggestions, or fixes, I would appreciate any [issue](https://github.com/notKitory/fuzix-sh/issues) or [pull request](https://github.com/notKitory/fuzix-sh/pulls).
