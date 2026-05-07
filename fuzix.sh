#!/bin/sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
STATE_DIR=${FUZIX_PLAYGROUND_DIR:-"$ROOT_DIR/.fuzix-playground"}
TOOL_DIR="$STATE_DIR/tools"
DOCKER_IMAGE=${FUZIX_DOCKER_IMAGE:-fuzix-playground:debian-trixie}
FUZIX_REBUILD_DOCKER=${FUZIX_REBUILD_DOCKER:-0}
FUZIX_REF=${FUZIX_REF:-v0.4}
FUZIX_REPO=${FUZIX_REPO:-https://github.com/EtchedPixels/FUZIX.git}
FUZIX_BINTOOLS_REPO=${FUZIX_BINTOOLS_REPO:-https://github.com/EtchedPixels/Fuzix-Bintools.git}
FUZIX_FCC_REPO=${FUZIX_FCC_REPO:-https://github.com/EtchedPixels/Fuzix-Compiler-Kit.git}
Z80PACK_REPO=${Z80PACK_REPO:-https://github.com/udo-munk/z80pack.git}
SDCC_REPO=${SDCC_REPO:-https://github.com/EtchedPixels/sdcc280.git}
FUZIX_IMAGE_BASE=${FUZIX_IMAGE_BASE:-https://fuzix.org/downloads/0.4/z80pack}
FUZIX_CPU=${FUZIX_CPU:-8080}
FUZIX_BUILD_FROM_SOURCE=${FUZIX_BUILD_FROM_SOURCE:-0}
TIMEOUT=${FUZIX_TIMEOUT:-45}
PATH=$TOOL_DIR/fcc/bin:$TOOL_DIR/sdcc/bin:/opt/fcc/bin:$PATH
export PATH

usage() {
    cat <<EOF
Usage:
  ./fuzix-playground.sh init
  ./fuzix-playground.sh setup
  ./fuzix-playground.sh compile <source.c> [program-name]
  ./fuzix-playground.sh image <program-name>
  ./fuzix-playground.sh run [-v] <program-name> [-- arg...]
  ./fuzix-playground.sh shell [program-name]
  ./fuzix-playground.sh test [-v] <source.c> [program-name] [-- arg...]

Environment:
  FUZIX_PLAYGROUND_DIR  state dir, default: ./.fuzix-playground
  FUZIX_REF             FUZIX git ref, default: v0.4
  FUZIX_CPU             compiler CPU, default: 8080
  FUZIX_BUILD_FROM_SOURCE=1  build FUZIX images instead of downloading release images
  FUZIX_REBUILD_DOCKER=1     force Docker image rebuild
  FUZIX_TIMEOUT         emulator check timeout, default: 45 seconds
EOF
}

need() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "Missing required command: $1" >&2
        exit 1
    }
}

host_notice() {
    echo "Fuzix playground: Ctrl-] force-exits the interactive emulator."
}

dockerfile() {
    mkdir -p "$STATE_DIR"
    cat >"$STATE_DIR/Dockerfile" <<'EOF'
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    bison \
    build-essential \
    automake \
    byacc \
    ca-certificates \
    curl \
    expect \
    flex \
    git \
    gputils \
    libboost-dev \
    libboost-program-options-dev \
    libbsd-dev \
    libjpeg-dev \
    make \
    perl \
    pkg-config \
    sudo \
    tcl \
    texinfo \
    wget \
    zlib1g-dev \
    && (update-alternatives --set yacc /usr/bin/byacc || true) \
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
    docker build -t "$DOCKER_IMAGE" "$STATE_DIR"
}

docker_run() {
    need docker
    mkdir -p "$STATE_DIR/opt-fcc"
    docker run --rm -i \
        -v "$ROOT_DIR:/workspace" \
        -v "$STATE_DIR/opt-fcc:/opt/fcc" \
        -w /workspace \
        -e FUZIX_PLAYGROUND_DIR=/workspace/.fuzix-playground \
        -e FUZIX_REF="$FUZIX_REF" \
        -e FUZIX_REPO="$FUZIX_REPO" \
        -e FUZIX_BINTOOLS_REPO="$FUZIX_BINTOOLS_REPO" \
        -e FUZIX_FCC_REPO="$FUZIX_FCC_REPO" \
        -e Z80PACK_REPO="$Z80PACK_REPO" \
        -e SDCC_REPO="$SDCC_REPO" \
        -e FUZIX_IMAGE_BASE="$FUZIX_IMAGE_BASE" \
        -e FUZIX_CPU="$FUZIX_CPU" \
        -e FUZIX_BUILD_FROM_SOURCE="$FUZIX_BUILD_FROM_SOURCE" \
        -e FUZIX_TIMEOUT="$TIMEOUT" \
        "$DOCKER_IMAGE" \
        /bin/sh /workspace/fuzix-playground.sh _inside "$@"
}

docker_run_tty() {
    need docker
    mkdir -p "$STATE_DIR/opt-fcc"
    docker run --rm -it \
        -v "$ROOT_DIR:/workspace" \
        -v "$STATE_DIR/opt-fcc:/opt/fcc" \
        -w /workspace \
        -e FUZIX_PLAYGROUND_DIR=/workspace/.fuzix-playground \
        -e FUZIX_REF="$FUZIX_REF" \
        -e FUZIX_REPO="$FUZIX_REPO" \
        -e FUZIX_BINTOOLS_REPO="$FUZIX_BINTOOLS_REPO" \
        -e FUZIX_FCC_REPO="$FUZIX_FCC_REPO" \
        -e Z80PACK_REPO="$Z80PACK_REPO" \
        -e SDCC_REPO="$SDCC_REPO" \
        -e FUZIX_IMAGE_BASE="$FUZIX_IMAGE_BASE" \
        -e FUZIX_CPU="$FUZIX_CPU" \
        -e FUZIX_BUILD_FROM_SOURCE="$FUZIX_BUILD_FROM_SOURCE" \
        -e FUZIX_TIMEOUT="$TIMEOUT" \
        "$DOCKER_IMAGE" \
        /bin/sh /workspace/fuzix-playground.sh _inside "$@"
}

git_clone_once() {
    url=$1
    dir=$2
    ref=${3:-}

    if [ -d "$dir/.git" ]; then
        return 0
    fi

    mkdir -p "$(dirname "$dir")"
    if [ -n "$ref" ]; then
        if git clone --depth 1 --branch "$ref" "$url" "$dir"; then
            return 0
        fi
        rm -rf "$dir"
        case "$ref" in
            v*) alt_ref=${ref#v} ;;
            *) alt_ref=v$ref ;;
        esac
        git clone --depth 1 --branch "$alt_ref" "$url" "$dir"
    else
        git clone --depth 1 "$url" "$dir"
    fi
}

inside_setup_sdcc() {
    if command -v sdcc >/dev/null 2>&1; then
        return 0
    fi

    git_clone_once "$SDCC_REPO" "$STATE_DIR/src/sdcc280" ""
    cd "$STATE_DIR/src/sdcc280/sdcc"
    if [ ! -f Makefile ]; then
        ./configure --prefix="$TOOL_DIR/sdcc" --disable-pic14-port --disable-pic16-port
    fi
    make
    make install
}

inside_fetch_images() {
    pristine_dir="$STATE_DIR/downloads/z80pack"
    pristine_boot="$pristine_dir/boot.dsk"
    pristine_hd="$pristine_dir/hd-fuzix.dsk"
    boot="$STATE_DIR/src/FUZIX/Images/z80pack/boot.dsk"
    hd="$STATE_DIR/src/FUZIX/Images/z80pack/hd-fuzix.dsk"

    mkdir -p "$pristine_dir"
    [ -s "$pristine_boot" ] || wget -O "$pristine_boot" "$FUZIX_IMAGE_BASE/boot.dsk"
    [ -s "$pristine_hd" ] || wget -O "$pristine_hd" "$FUZIX_IMAGE_BASE/hd-fuzix.dsk"

    mkdir -p "$(dirname "$boot")"
    cp "$pristine_boot" "$boot"
    cp "$pristine_hd" "$hd"
}

inside_setup_fcc() {
    if { command -v fcc >/dev/null 2>&1 || [ -x /opt/fcc/bin/cc ]; } \
        && command -v asz80 >/dev/null 2>&1 \
        && command -v ldz80 >/dev/null 2>&1; then
        if [ -x /opt/fcc/bin/fcc ]; then
            ln -sf fcc /opt/fcc/bin/cc85
        fi
        return 0
    fi

    git_clone_once "$FUZIX_BINTOOLS_REPO" "$STATE_DIR/src/Fuzix-Bintools" ""
    cd "$STATE_DIR/src/Fuzix-Bintools"
    make install

    git_clone_once "$FUZIX_FCC_REPO" "$STATE_DIR/src/Fuzix-Compiler-Kit" ""
    cd "$STATE_DIR/src/Fuzix-Compiler-Kit"
    make bootstuff
    make install

    ln -sf fcc /opt/fcc/bin/cc85
}

inside_setup_fuzix() {
    git_clone_once "$FUZIX_REPO" "$STATE_DIR/src/FUZIX" "$FUZIX_REF"

    cd "$STATE_DIR/src/FUZIX"
    if [ "$FUZIX_BUILD_FROM_SOURCE" = 1 ]; then
        perl -0pi -e 's/^TARGET\s*\??=.*$/TARGET ?= z80pack/m' Makefile

        make
        make diskimage
    else
        inside_fetch_images
    fi

    if [ -f Standalone/Makefile ] && [ ! -x Standalone/ucp ]; then
        make -C Standalone ucp
    fi
}

inside_setup_userlib() {
    fuzix="$STATE_DIR/src/FUZIX"
    libs="$fuzix/Library/libs"

    ln -sf fcc /opt/fcc/bin/cc85
    if [ ! -x "$fuzix/Library/tools/binman85" ] \
        || [ ! -x "$fuzix/Library/tools/liberror" ] \
        || [ ! -x "$fuzix/Library/tools/syscall_8080" ]; then
        make -C "$fuzix/Library" tools/binman85 tools/liberror tools/syscall_8080
    fi

    if [ ! -s "$libs/libc8080.a" ] || [ ! -s "$libs/crt0_8080.o" ]; then
        (
            cd "$libs"
            make -f Makefile.8080
        )
    fi
}

inside_setup_z80pack() {
    git_clone_once "$Z80PACK_REPO" "$STATE_DIR/src/z80pack" ""

    cd "$STATE_DIR/src/z80pack"
    if [ -x cpmsim/cpmsim ]; then
        if [ -d cpmsim/srctools ] && [ ! -x cpmsim/srctools/cpmrecv ]; then
            make -C cpmsim/srctools
        fi
        return 0
    fi

    if [ -d cpmsim/srcsim ]; then
        (
            cd cpmsim/srcsim
            make || make -f Makefile.linux || make -f Makefile.linuxc
        )
    fi

    if [ ! -x cpmsim/cpmsim ]; then
        make
    fi

    if [ -d cpmsim/srctools ] && [ ! -x cpmsim/srctools/cpmrecv ]; then
        make -C cpmsim/srctools
    fi
}

inside_setup() {
    mkdir -p "$STATE_DIR/bin" "$STATE_DIR/images" "$STATE_DIR/src"
    inside_setup_fcc
    inside_setup_fuzix
    inside_setup_userlib
    inside_setup_z80pack
}

program_name_from_source() {
    base=$(basename -- "$1")
    printf '%s\n' "${base%.*}"
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

inside_compile() {
    src=$1
    name=${2:-$(program_name_from_source "$src")}

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
        echo "No FUZIX compiler driver found. Run setup first." >&2
        exit 1
    fi

    build_dir="$STATE_DIR/build/$name"
    fuzix="$STATE_DIR/src/FUZIX"
    if [ -e "$build_dir" ] && [ ! -d "$build_dir" ]; then
        rm -f "$build_dir"
    fi
    mkdir -p "$build_dir"

    export FUZIX_ROOT="$fuzix"
    "$compiler" "-m$FUZIX_CPU" -Os -D__STDC__ -c \
        -I"$STATE_DIR/src/FUZIX/Library/include" \
        -I"$STATE_DIR/src/FUZIX/Library/include/$FUZIX_CPU" \
        "$source_path" -o "$build_dir/$name.o"

    "$compiler" -s "-m$FUZIX_CPU" \
        "$fuzix/Library/libs/crt0_${FUZIX_CPU}.o" \
        "$build_dir/$name.o" \
        -o "$build_dir/$name.b1" -M \
        -L"$fuzix/Library/libs" \
        "-lc$FUZIX_CPU" "-lc$FUZIX_CPU" "-lc$FUZIX_CPU"

    cp "$build_dir/$name.b1" "$out"
    "$fuzix/Library/tools/binman85" "$out"

    [ -s "$out" ] || {
        echo "Compiler did not create $out" >&2
        exit 1
    }

    echo "Compiled: $out"
}

find_ucp() {
    for p in \
        "$STATE_DIR/src/FUZIX/Standalone/filesystem-src/ucp" \
        "$STATE_DIR/src/FUZIX/Standalone/ucp"
    do
        if [ -x "$p" ]; then
            printf '%s\n' "$p"
            return 0
        fi
    done

    find "$STATE_DIR/src/FUZIX" -type f -name ucp -perm -111 | head -n 1
}

inside_image() {
    name=$1
    app="$STATE_DIR/bin/$name"
    boot="$STATE_DIR/src/FUZIX/Images/z80pack/boot.dsk"
    hd="$STATE_DIR/src/FUZIX/Images/z80pack/hd-fuzix.dsk"
    out_boot="$STATE_DIR/images/boot.dsk"
    out_hd="$STATE_DIR/images/hd-fuzix-$name.dsk"

    [ -s "$app" ] || {
        echo "Program binary not found: $app" >&2
        echo "Run: ./fuzix-playground.sh compile <source.c> $name" >&2
        exit 1
    }
    [ -s "$boot" ] || {
        echo "Boot image not found. Run setup first." >&2
        exit 1
    }
    [ -s "$hd" ] || {
        echo "Root filesystem image not found. Run setup first." >&2
        exit 1
    }

    ucp=$(find_ucp)
    [ -n "$ucp" ] || {
        echo "ucp not found. Run setup first." >&2
        exit 1
    }

    mkdir -p "$STATE_DIR/images"
    cp "$boot" "$out_boot"
    cp "$hd" "$out_hd"

    (
        cd "$STATE_DIR/bin"
        printf 'cd /bin\nbget %s\nchmod 0755 %s\nexit\n' "$name" "$name" | "$ucp" "$out_hd"
    )

    echo "Boot image: $out_boot"
    echo "Root image: $out_hd"
}

inside_run() {
    verbose=$1
    name=$2
    shift 2
    boot="$STATE_DIR/images/boot.dsk"
    hd="$STATE_DIR/images/hd-fuzix-$name.dsk"
    z80="$STATE_DIR/src/z80pack/cpmsim"
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
        echo "cpmsim not found. Run setup first." >&2
        exit 1
    }

    mkdir -p "$z80/disks"
    cp "$boot" "$z80/disks/drivea.dsk"
    cp "$hd" "$z80/disks/drivei.dsk"

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
    name=${1:-}
    boot="$STATE_DIR/images/boot.dsk"
    base_hd="$STATE_DIR/src/FUZIX/Images/z80pack/hd-fuzix.dsk"
    z80="$STATE_DIR/src/z80pack/cpmsim"

    if [ -n "$name" ]; then
        hd="$STATE_DIR/images/hd-fuzix-$name.dsk"
    else
        hd="$STATE_DIR/images/hd-fuzix-shell.dsk"
        cp "$base_hd" "$hd"
    fi

    [ -s "$boot" ] || cp "$STATE_DIR/src/FUZIX/Images/z80pack/boot.dsk" "$boot"
    [ -s "$hd" ] || {
        echo "Root image not found. Run setup first, or image <program-name> for app image." >&2
        exit 1
    }
    [ -x "$z80/cpmsim" ] || {
        echo "cpmsim not found. Run setup first." >&2
        exit 1
    }

    mkdir -p "$z80/disks"
    cp "$boot" "$z80/disks/drivea.dsk"
    cp "$hd" "$z80/disks/drivei.dsk"

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

host_init() {
    mkdir -p "$ROOT_DIR/programs"
    sample="$ROOT_DIR/programs/hello.c"
    if [ ! -f "$sample" ]; then
        cat >"$sample" <<'EOF'
#include <stdio.h>

int main(int argc, char **argv)
{
    argc = argc;
    argv = argv;
    puts("hello from fuzix");
    return 0;
}
EOF
    fi
    echo "Sample source: $sample"
}

cmd=${1:-}
case "$cmd" in
    init)
        host_notice
        host_init
        ;;
    setup)
        host_notice
        host_docker_build
        docker_run setup
        ;;
    compile)
        [ $# -ge 2 ] || { usage; exit 1; }
        host_notice
        docker_run compile "$2" "${3:-}"
        ;;
    image)
        [ $# -eq 2 ] || { usage; exit 1; }
        host_notice
        docker_run image "$2"
        ;;
    run)
        shift
        verbose=0
        if [ "${1:-}" = "-v" ]; then
            verbose=1
            shift
        fi
        [ $# -ge 1 ] || { usage; exit 1; }
        [ "$verbose" = 1 ] && host_notice
        name=$1
        shift
        if [ "${1:-}" = "--" ]; then
            shift
        elif [ $# -gt 0 ]; then
            usage
            exit 1
        fi
        docker_run run "$verbose" "$name" "$@"
        ;;
    shell)
        host_notice
        host_docker_build
        docker_run setup
        docker_run_tty shell "${2:-}"
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
        if [ "$verbose" = 1 ]; then
            host_docker_build
        else
            host_docker_build >/dev/null
        fi
        src=$1
        shift
        if [ "${1:-}" = "--" ]; then
            name=$(program_name_from_source "$src")
            shift
        else
            name=${1:-$(program_name_from_source "$src")}
            [ $# -gt 0 ] && shift
            if [ "${1:-}" = "--" ]; then
                shift
            elif [ $# -gt 0 ]; then
                usage
                exit 1
            fi
        fi
        if [ "$verbose" = 1 ]; then
            docker_run setup
            docker_run compile "$src" "$name"
            docker_run image "$name"
        else
            docker_run setup >/dev/null
            docker_run compile "$src" "$name" >/dev/null
            docker_run image "$name" >/dev/null
        fi
        docker_run run "$verbose" "$name" "$@"
        ;;
    _inside)
        shift
        inside_cmd=${1:-}
        case "$inside_cmd" in
            setup)
                inside_setup
                ;;
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
