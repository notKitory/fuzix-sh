#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
STATE_DIR=${FUZIX_SH_DIR:-"$ROOT_DIR/.fuzix-sh"}
FUZIX_CPU=${FUZIX_CPU:-8080}
TIMEOUT=${FUZIX_TIMEOUT:-45}
FUZIX_REBUILD_DOCKER=${FUZIX_REBUILD_DOCKER:-0}
FUZIX_PREBUILT_REPO=${FUZIX_PREBUILT_REPO:-notKitory/fuzix-sh}
FUZIX_PREBUILT_RELEASE=${FUZIX_PREBUILT_RELEASE:-latest}
FUZIX_PREBUILT_ARCH=${FUZIX_PREBUILT_ARCH:-}

detect_prebuilt_arch() {
    if [ -n "$FUZIX_PREBUILT_ARCH" ]; then
        case "$FUZIX_PREBUILT_ARCH" in
            linux-amd64|linux-arm64)
                printf '%s\n' "$FUZIX_PREBUILT_ARCH"
                return 0
                ;;
            *)
                echo "Unsupported FUZIX_PREBUILT_ARCH: $FUZIX_PREBUILT_ARCH" >&2
                echo "Supported: linux-amd64, linux-arm64" >&2
                exit 1
                ;;
        esac
    fi

    case "$(uname -m)" in
        x86_64|amd64) printf '%s\n' linux-amd64 ;;
        arm64|aarch64) printf '%s\n' linux-arm64 ;;
        *)
            echo "Unsupported host architecture: $(uname -m)" >&2
            echo "Set FUZIX_PREBUILT_ARCH to linux-amd64 or linux-arm64." >&2
            exit 1
            ;;
    esac
}

PREBUILT_ARCH=$(detect_prebuilt_arch)
PREBUILT_DIR="$STATE_DIR/prebuilt/$PREBUILT_ARCH"
DOCKER_PLATFORM=
case "$PREBUILT_ARCH" in
    linux-amd64) DOCKER_PLATFORM=linux/amd64 ;;
    linux-arm64) DOCKER_PLATFORM=linux/arm64 ;;
esac
DOCKER_IMAGE=${FUZIX_DOCKER_IMAGE:-fuzix-sh:debian-trixie-$PREBUILT_ARCH}
PATH=/opt/fcc/bin:$PATH
export PATH

usage() {
    cat <<EOF
Usage:
  ./fuzix.sh compile <source.c>
  ./fuzix.sh image
  ./fuzix.sh run [-v] [arg...]
  ./fuzix.sh shell
  ./fuzix.sh test [-v] <source.c> [arg...]

Environment:
  FUZIX_SH_DIR    state dir, default: ./.fuzix-sh
  FUZIX_CPU               compiler CPU, default: 8080
  FUZIX_DOCKER_IMAGE      runtime Docker image name
  FUZIX_REBUILD_DOCKER=1  force Docker image rebuild
  FUZIX_TIMEOUT           emulator check timeout, default: 45 seconds
  FUZIX_PREBUILT_REPO     GitHub repo with release assets, default: notKitory/fuzix-sh
  FUZIX_PREBUILT_RELEASE  release tag or latest, default: latest
  FUZIX_PREBUILT_ARCH     linux-amd64 or linux-arm64
EOF
}

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "Missing required command: $1" >&2
        exit 1
    }
}

host_notice() {
    echo "Ctrl-] force-shutdowns the emulator."
}

release_asset_url() {
    asset=$1
    if [ "$FUZIX_PREBUILT_RELEASE" = latest ]; then
        printf 'https://github.com/%s/releases/latest/download/%s\n' \
            "$FUZIX_PREBUILT_REPO" "$asset"
    else
        printf 'https://github.com/%s/releases/download/%s/%s\n' \
            "$FUZIX_PREBUILT_REPO" "$FUZIX_PREBUILT_RELEASE" "$asset"
    fi
}

validate_prebuilt() {
    dir=$1
    missing=0
    for path in \
        "$dir/opt-fcc/bin/fcc" \
        "$dir/opt-fcc/bin/asz80" \
        "$dir/opt-fcc/bin/ldz80" \
        "$dir/fuzix/include/stdio.h" \
        "$dir/fuzix/include/$FUZIX_CPU" \
        "$dir/fuzix/libs/crt0_${FUZIX_CPU}.o" \
        "$dir/fuzix/libs/libc${FUZIX_CPU}.a" \
        "$dir/fuzix/tools/binman85" \
        "$dir/fuzix/ucp" \
        "$dir/fuzix/images/boot.dsk" \
        "$dir/fuzix/images/hd-fuzix.dsk" \
        "$dir/z80pack/cpmsim/cpmsim" \
        "$dir/z80pack/cpmsim/srctools"
    do
        if [ ! -e "$path" ]; then
            echo "Missing prebuilt file: $path" >&2
            missing=1
        fi
    done
    [ "$missing" = 0 ]
}

ensure_prebuilt() {
    need curl
    need tar

    if [ -f "$PREBUILT_DIR/.complete" ] && validate_prebuilt "$PREBUILT_DIR" >/dev/null 2>&1; then
        return 0
    fi

    mkdir -p "$STATE_DIR/prebuilt"
    tmp="$PREBUILT_DIR.tmp.$$"
    rm -rf "$tmp"
    mkdir -p "$tmp"

    for asset in \
        "fuzix-toolchain-$PREBUILT_ARCH.tar.gz" \
        "fuzix-runtime-$PREBUILT_ARCH.tar.gz" \
        "z80pack-runtime-$PREBUILT_ARCH.tar.gz"
    do
        url=$(release_asset_url "$asset")
        archive="$tmp/$asset"
        echo "Downloading $url"
        if ! curl -fL --retry 3 -o "$archive" "$url"; then
            echo "Could not download release asset: $asset" >&2
            echo "Repo: $FUZIX_PREBUILT_REPO, release: $FUZIX_PREBUILT_RELEASE, arch: $PREBUILT_ARCH" >&2
            echo "Override with FUZIX_PREBUILT_REPO, FUZIX_PREBUILT_RELEASE, or FUZIX_PREBUILT_ARCH." >&2
            rm -rf "$tmp"
            exit 1
        fi
        tar -xzf "$archive" -C "$tmp"
    done

    validate_prebuilt "$tmp" || {
        rm -rf "$tmp"
        exit 1
    }

    rm -rf "$PREBUILT_DIR"
    mv "$tmp" "$PREBUILT_DIR"
    date -u '+%Y-%m-%dT%H:%M:%SZ' >"$PREBUILT_DIR/.complete"
}

dockerfile() {
    mkdir -p "$STATE_DIR"
    cat >"$STATE_DIR/Dockerfile" <<'EOF'
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    expect \
    perl \
    && rm -rf /var/lib/apt/lists/*

ENV PATH=/opt/fcc/bin:/usr/local/bin:/usr/bin:/bin
WORKDIR /workspace
EOF
}

host_docker_build() {
    need docker
    if [ "$FUZIX_REBUILD_DOCKER" != 1 ] && docker image inspect "$DOCKER_IMAGE" >/dev/null 2>&1; then
        return 0
    fi

    dockerfile
    docker build --platform "$DOCKER_PLATFORM" -t "$DOCKER_IMAGE" "$STATE_DIR"
}

docker_run() {
    need docker
    ensure_prebuilt
    host_docker_build
    docker run --rm -i --platform "$DOCKER_PLATFORM" \
        -v "$ROOT_DIR:/workspace" \
        -v "$PREBUILT_DIR/opt-fcc:/opt/fcc" \
        -w /workspace \
        -e FUZIX_SH_DIR=/workspace/.fuzix-sh \
        -e FUZIX_PREBUILT_DIR="/workspace/.fuzix-sh/prebuilt/$PREBUILT_ARCH" \
        -e FUZIX_CPU="$FUZIX_CPU" \
        -e FUZIX_TIMEOUT="$TIMEOUT" \
        "$DOCKER_IMAGE" \
        /bin/sh /workspace/fuzix.sh _inside "$@"
}

docker_run_tty() {
    need docker
    ensure_prebuilt
    host_docker_build
    docker run --rm -it --platform "$DOCKER_PLATFORM" \
        -v "$ROOT_DIR:/workspace" \
        -v "$PREBUILT_DIR/opt-fcc:/opt/fcc" \
        -w /workspace \
        -e FUZIX_SH_DIR=/workspace/.fuzix-sh \
        -e FUZIX_PREBUILT_DIR="/workspace/.fuzix-sh/prebuilt/$PREBUILT_ARCH" \
        -e FUZIX_CPU="$FUZIX_CPU" \
        -e FUZIX_TIMEOUT="$TIMEOUT" \
        "$DOCKER_IMAGE" \
        /bin/sh /workspace/fuzix.sh _inside "$@"
}

program_name_from_source() {
    base=$(basename -- "$1")
    printf '%s\n' "${base%.*}"
}

current_program_name() {
    file="$STATE_DIR/current-program"
    [ -s "$file" ] || {
        echo "No current program. Run: ./fuzix.sh compile <source.c>" >&2
        exit 1
    }
    IFS= read -r name <"$file" || name=
    [ -n "$name" ] || {
        echo "Current program file is empty: $file" >&2
        exit 1
    }
    printf '%s\n' "$name"
}

validate_fuzix_token() {
    case "$1" in
        ""|*[!abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_./:+=,@%-]*)
            echo "Unsupported Fuzix shell token: $1" >&2
            echo "Use only letters, digits, and _ . / : + = , @ % -" >&2
            exit 1
            ;;
    esac
}

inside_prebuilt_dir() {
    dir=${FUZIX_PREBUILT_DIR:-}
    [ -n "$dir" ] || {
        echo "FUZIX_PREBUILT_DIR is not set inside the container." >&2
        exit 1
    }
    [ -d "$dir" ] || {
        echo "Prebuilt runtime not found: $dir" >&2
        exit 1
    }
    printf '%s\n' "$dir"
}

inside_ensure_images() {
    prebuilt=$(inside_prebuilt_dir)
    mkdir -p "$STATE_DIR/images"
    [ -s "$STATE_DIR/images/boot.dsk" ] || cp "$prebuilt/fuzix/images/boot.dsk" "$STATE_DIR/images/boot.dsk"
    [ -s "$STATE_DIR/images/hd-fuzix.dsk" ] || cp "$prebuilt/fuzix/images/hd-fuzix.dsk" "$STATE_DIR/images/hd-fuzix.dsk"
}

inside_compile() {
    src=$1
    name=$(program_name_from_source "$src")
    prebuilt=$(inside_prebuilt_dir)

    [ -f "$src" ] || {
        echo "Source file not found: $src" >&2
        exit 1
    }

    mkdir -p "$STATE_DIR/bin"
    out="$STATE_DIR/bin/$name"
    case "$src" in
        /*) source_path=$src ;;
        *) source_path=/workspace/$src ;;
    esac

    cd "$STATE_DIR"
    rm -f "$out"

    if command -v cc85 >/dev/null 2>&1; then
        compiler=$(command -v cc85)
    elif command -v fcc >/dev/null 2>&1; then
        compiler=$(command -v fcc)
    else
        echo "No FUZIX compiler driver found in /opt/fcc/bin." >&2
        exit 1
    fi

    build_dir="$STATE_DIR/build/$name"
    if [ -e "$build_dir" ] && [ ! -d "$build_dir" ]; then
        rm -f "$build_dir"
    fi
    mkdir -p "$build_dir"

    export FUZIX_ROOT="$prebuilt/fuzix"
    "$compiler" "-m$FUZIX_CPU" -Os -D__STDC__ -c \
        -I"$prebuilt/fuzix/include" \
        -I"$prebuilt/fuzix/include/$FUZIX_CPU" \
        "$source_path" -o "$build_dir/$name.o"

    "$compiler" -s "-m$FUZIX_CPU" \
        "$prebuilt/fuzix/libs/crt0_${FUZIX_CPU}.o" \
        "$build_dir/$name.o" \
        -o "$build_dir/$name.b1" -M \
        -L"$prebuilt/fuzix/libs" \
        "-lc$FUZIX_CPU" "-lc$FUZIX_CPU" "-lc$FUZIX_CPU"

    cp "$build_dir/$name.b1" "$out"
    "$prebuilt/fuzix/tools/binman85" "$out"

    [ -s "$out" ] || {
        echo "Compiler did not create $out" >&2
        exit 1
    }

    printf '%s\n' "$name" >"$STATE_DIR/current-program"
    echo "Compiled: $out"
}

inside_image() {
    name=$(current_program_name)
    prebuilt=$(inside_prebuilt_dir)
    app="$STATE_DIR/bin/$name"
    out_boot="$STATE_DIR/images/boot.dsk"
    out_hd="$STATE_DIR/images/hd-fuzix.dsk"
    ucp="$prebuilt/fuzix/ucp"

    [ -s "$app" ] || {
        echo "Program binary not found: $app" >&2
        echo "Run: ./fuzix.sh compile <source.c> $name" >&2
        exit 1
    }
    [ -x "$ucp" ] || {
        echo "ucp not found in prebuilt runtime: $ucp" >&2
        exit 1
    }

    inside_ensure_images

    (
        cd "$STATE_DIR/bin"
        printf 'cd /bin\nbget %s\nchmod 0755 %s\nexit\n' "$name" "$name" | "$ucp" "$out_hd"
    )

    echo "Boot image: $out_boot"
    echo "Root image: $out_hd"
}

inside_link_disks() {
    z80=$1
    boot=$2
    hd=$3

    mkdir -p "$z80/disks"
    ln -sf "$boot" "$z80/disks/drivea.dsk"
    ln -sf "$hd" "$z80/disks/drivei.dsk"
}

inside_run() {
    verbose=$1
    name=$2
    shift 2
    prebuilt=$(inside_prebuilt_dir)
    boot="$STATE_DIR/images/boot.dsk"
    hd="$STATE_DIR/images/hd-fuzix.dsk"
    z80="$prebuilt/z80pack/cpmsim"
    run_line="/bin/$name"

    validate_fuzix_token "$name"
    for arg in "$@"; do
        validate_fuzix_token "$arg"
        run_line="$run_line $arg"
    done

    [ -s "$boot" ] || {
        echo "Boot image not found. Run image first." >&2
        exit 1
    }
    [ -s "$hd" ] || {
        echo "Root image not found. Run image first." >&2
        exit 1
    }
    [ -x "$z80/cpmsim" ] || {
        echo "cpmsim not found in prebuilt runtime: $z80/cpmsim" >&2
        exit 1
    }

    inside_link_disks "$z80" "$boot" "$hd"

    expect_file="$STATE_DIR/run-$name.expect"
    log_file="$STATE_DIR/run-$name.log"
    cat >"$expect_file" <<EOF
set timeout $TIMEOUT
log_user $verbose
log_file -a -noappend "$log_file"
cd "$z80"
set env(PATH) "$z80/srctools:\$env(PATH)"
proc finish_emulator {status} {
    catch {send "\034"}
    after 500
    catch {close}
    catch {wait}
    exit \$status
}
proc shutdown_then_finish {status} {
    catch {send "shutdown\r"}
    set old_timeout \$::timeout
    set timeout 10
    expect {
        -re {halt|Halted|System halted} { }
        timeout { }
    }
    set timeout \$old_timeout
    finish_emulator \$status
}
spawn ./cpmsim
expect {
    -re {bootdev:} { send "0\r" }
    timeout { puts "timeout waiting for bootdev"; finish_emulator 2 }
}
expect {
    -re {login:} { send "root\r" }
    timeout { puts "timeout waiting for login"; finish_emulator 2 }
}
expect {
    -re {[\$#] } { send "$run_line\r" }
    timeout { puts "timeout waiting for shell"; finish_emulator 2 }
}
EOF

    cat >>"$expect_file" <<'EOF'
expect {
    -re {[$#] } { }
    timeout { puts "timeout waiting for program to return"; finish_emulator 3 }
}
EOF

    cat >>"$expect_file" <<'EOF'
send "shutdown\r"
set timeout 10
expect {
    -re {halt|Halted|System halted} { }
    timeout { }
}
finish_emulator 0
EOF

    expect "$expect_file"
    if [ "$verbose" = 1 ]; then
        echo "Emulator log: $log_file"
    else
        RUN_LINE="$run_line" perl -0ne '
            my $line = $ENV{RUN_LINE};
            my $start = index($_, "$line\r\n");
            $start = index($_, "$line\n") if $start < 0;
            exit 0 if $start < 0;
            $start += length($line);
            $start++ if substr($_, $start, 1) eq "\r";
            $start++ if substr($_, $start, 1) eq "\n";
            my $out = substr($_, $start);
            my $end = index($out, "\r\n# shutdown");
            $end = index($out, "\n# shutdown") if $end < 0;
            $end = index($out, "\r\n# ") if $end < 0;
            $end = index($out, "\n# ") if $end < 0;
            $out = substr($out, 0, $end) if $end >= 0;
            $out =~ s/\r\n/\n/g;
            $out =~ s/\r/\n/g;
            print $out;
            print "\n" if length($out) && $out !~ /\n\z/;
        ' "$log_file"
    fi
}

inside_shell() {
    prebuilt=$(inside_prebuilt_dir)
    boot="$STATE_DIR/images/boot.dsk"
    hd="$STATE_DIR/images/hd-fuzix.dsk"
    z80="$prebuilt/z80pack/cpmsim"

    inside_ensure_images
    [ -x "$z80/cpmsim" ] || {
        echo "cpmsim not found in prebuilt runtime: $z80/cpmsim" >&2
        exit 1
    }

    inside_link_disks "$z80" "$boot" "$hd"

    expect_file="$STATE_DIR/shell.expect"
    cat >"$expect_file" <<EOF
set timeout 60
cd "$z80"
set env(PATH) "$z80/srctools:\$env(PATH)"
set key_interrupt "\034"
set key_shutdown "\035"
proc restore_tty {} {
    catch {stty sane}
}
proc finish_emulator {status} {
    restore_tty
    catch {send "\034"}
    after 500
    catch {close}
    catch {wait}
    exit \$status
}
trap {
    finish_emulator 130
} SIGINT
trap {
    finish_emulator 143
} SIGTERM
spawn ./cpmsim
expect {
    -re {bootdev:} { send "0\r" }
    timeout { puts "timeout waiting for bootdev"; exit 2 }
}
expect {
    -re {login:} { send "root\r" }
    timeout { puts "timeout waiting for login"; exit 2 }
}
expect {
    -re {[\$#] } {
        log_user 0
        send "stty erase '^?'\r"
        expect -re {[\$#] }
        log_user 1
        interact \
            -o -re {Halted\.|System halted|halt:} {
                finish_emulator 0
            } \
            \$key_shutdown {
                finish_emulator 0
            }
        restore_tty
    }
    timeout { puts "timeout waiting for shell"; exit 2 }
}
EOF

    expect "$expect_file"
}

cmd=${1:-}
case "$cmd" in
    compile)
        [ $# -eq 2 ] || { usage; exit 1; }
        docker_run compile "$2"
        ;;
    image)
        [ $# -eq 1 ] || { usage; exit 1; }
        docker_run image
        ;;
    run)
        shift
        verbose=0
        if [ "${1:-}" = "-v" ]; then
            verbose=1
            shift
        fi
        [ "$verbose" = 1 ] && host_notice
        if [ "${1:-}" = "--" ]; then
            shift
        fi
        name=$(current_program_name)
        docker_run run "$verbose" "$name" "$@"
        ;;
    shell)
        [ $# -eq 1 ] || { usage; exit 1; }
        host_notice
        docker_run_tty shell
        ;;
    test)
        shift
        verbose=0
        if [ "${1:-}" = "-v" ]; then
            verbose=1
            shift
        fi
        [ $# -ge 1 ] || { usage; exit 1; }
        [ "$verbose" = 1 ] && host_notice
        src=$1
        shift
        if [ "${1:-}" = "--" ]; then
            shift
        fi
        name=$(program_name_from_source "$src")
        if [ "$verbose" = 1 ]; then
            docker_run compile "$src"
            docker_run image
        else
            docker_run compile "$src" >/dev/null
            docker_run image >/dev/null
        fi
        docker_run run "$verbose" "$name" "$@"
        ;;
    _inside)
        shift
        inside_cmd=${1:-}
        case "$inside_cmd" in
            compile)
                shift
                inside_compile "$@"
                ;;
            image)
                shift
                inside_image "$@"
                ;;
            run)
                shift
                inside_run "$@"
                ;;
            shell)
                shift
                inside_shell "$@"
                ;;
            *)
                usage
                exit 1
                ;;
        esac
        ;;
    ""|-h|--help|help)
        usage
        ;;
    *)
        usage
        exit 1
        ;;
esac
