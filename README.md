# fuzix-sh

[English](./README.md) | [Русский](./README_RU.md)

`fuzix-sh` is a small shell wrapper for compiling and running C programs for FUZIX through z80pack.

## Requirements

- Docker
- `curl`
- `tar`

## Usage

To use downloading only [`fuzix.sh`](https://raw.githubusercontent.com/notKitory/fuzix-sh/main/fuzix.sh) is enough.

```bash
./fuzix.sh compile <source.c>
./fuzix.sh cp <host-path> <fuzix-path>
./fuzix.sh make [target...]
./fuzix.sh make-test [-v] [arg...]
./fuzix.sh run [-v] <command> [arg...]
./fuzix.sh shell
./fuzix.sh test [-v] <source.c> [arg...]
```

On the first run, the script downloads binaries into `.fuzix-sh/prebuilt/<arch>` and builds a Docker runtime image. Later runs reuse this runtime.

## Commands

| Command | Description |
| --- | --- |
| `compile <source.c>` | Compiles a C source file into `.fuzix-sh/bin/<source-name>`. |
| `cp <host-path> <fuzix-path>` | Copies a local file into the FUZIX root disk at `<fuzix-path>`. |
| `make [target...]` | Runs `make` in the FUZIX toolchain environment. |
| `make-test [-v] [arg...]` | Runs `make`, copies the first Makefile target to `/bin/<target-name>`, and runs it in FUZIX. |
| `run [-v] <command> [arg...]` | Boots FUZIX in z80pack, runs the command in the FUZIX shell, prints the command output, and shuts the emulator down. |
| `shell` | Opens an interactive FUZIX shell. |
| `test [-v] <source.c> [arg...]` | Runs `compile`, `cp`, and `run` in sequence. |

Use `-v` to show emulator output instead of only the program output.
`Ctrl-]` force-shuts down the emulator in `run` and `shell`.

Command arguments are passed directly after the command:

```bash
./fuzix.sh run ls /bin
./fuzix.sh run /bin/hello arg1 arg2
./fuzix.sh test hello.c arg1 arg2
./fuzix.sh make-test arg1 arg2
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

`hd-fuzix.dsk` is a single mutable root disk. Every `cp` copies the requested file into the same disk image.

## Development

If you have ideas, suggestions, or fixes, I would appreciate any [issue](https://github.com/notKitory/fuzix-sh/issues) or [pull request](https://github.com/notKitory/fuzix-sh/pulls).
