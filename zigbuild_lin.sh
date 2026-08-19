#!/usr/bin/env bash

set -euo pipefail

TARGET_TRIPLE="x86_64-unknown-linux-gnu"
GLIBC_VERSION="2.31"
ZIG_TARGET="$TARGET_TRIPLE.$GLIBC_VERSION"

if ! command -v zig >/dev/null 2>&1; then
    echo "Error: Zig is not installed." >&2
    echo "On Arch Linux: sudo pacman -S zig" >&2
    exit 1
fi

if ! command -v cargo-zigbuild >/dev/null 2>&1; then
    echo "Installing cargo-zigbuild..."
    cargo install --locked cargo-zigbuild
fi

# Keep Zig's cache in the project rather than depending on a particular
# writable home directory.
export ZIG_GLOBAL_CACHE_DIR="$PWD/target/zig-cache"
export RUSTFLAGS="${RUSTFLAGS:-} -A linker_messages"
# export DONT_USE_GENERATED_BINDINGS=1

cargo zigbuild --release --lib --target "$ZIG_TARGET"

PLUGIN="target/$TARGET_TRIPLE/release/libemojikanban.so"

if [[ ! -f "$PLUGIN" ]]; then
    echo "Error: Expected plugin was not produced at $PLUGIN" >&2
    exit 1
fi

# echo
# echo "Required glibc symbol versions:"

# objdump -T "$PLUGIN" |
#     grep -oE 'GLIBC_[0-9.]+' |
#     sort -Vu

# echo
# echo "Built $PLUGIN"
