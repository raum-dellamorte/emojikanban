#!/usr/bin/env bash

set -euo pipefail

OBS_ID="com.obsproject.Studio"

flatpak_require() {
    local ref="$1"

    if flatpak info "$ref" >/dev/null 2>&1; then
        echo "Found $ref"
    else
        echo "Installing $ref..."
        flatpak install --assumeyes flathub "$ref"
    fi
}

if ! command -v flatpak >/dev/null 2>&1; then
    echo "Error: Flatpak is not installed." >&2
    exit 1
fi

# Ensure the OBS Flatpak is installed.
flatpak_require "$OBS_ID"

# Example:
# org.freedesktop.Platform/x86_64/25.08
OBS_RUNTIME="$(flatpak info --show-runtime "$OBS_ID")"

IFS="/" read -r PLATFORM_ID SDK_ARCH SDK_BRANCH <<< "$OBS_RUNTIME"

if [[ -z "$PLATFORM_ID" || -z "$SDK_ARCH" || -z "$SDK_BRANCH" ]]; then
    echo "Error: Could not parse OBS runtime: $OBS_RUNTIME" >&2
    exit 1
fi

case "$PLATFORM_ID" in
    *.Platform)
        SDK_ID="${PLATFORM_ID%.Platform}.Sdk"
        ;;
    *)
        echo "Error: Unexpected OBS platform ID: $PLATFORM_ID" >&2
        exit 1
        ;;
esac

SDK_REF="$SDK_ID/$SDK_ARCH/$SDK_BRANCH"
RUST_REF="org.freedesktop.Sdk.Extension.rust-nightly/$SDK_ARCH/$SDK_BRANCH"

# Select the newest LLVM extension available for this SDK branch.
LLVM_REF=""
LLVM_NAME=""

# Try newer LLVM extensions first.
for LLVM_MAJOR in {25..17}; do
    CANDIDATE_NAME="llvm$LLVM_MAJOR"
    CANDIDATE_ID="org.freedesktop.Sdk.Extension.$CANDIDATE_NAME"
    CANDIDATE_REF="$CANDIDATE_ID/$SDK_ARCH/$SDK_BRANCH"

    if flatpak remote-info flathub "$CANDIDATE_REF" >/dev/null 2>&1; then
        LLVM_NAME="$CANDIDATE_NAME"
        LLVM_REF="$CANDIDATE_REF"
        break
    fi
done

if [[ -z "$LLVM_REF" ]]; then
    echo "Error: No compatible LLVM SDK extension found for $SDK_BRANCH." >&2
    exit 1
fi

LLVM_PATH="/usr/lib/sdk/$LLVM_NAME"

if [[ -z "$LLVM_REF" ]]; then
    echo "Error: No LLVM SDK extension found for $SDK_BRANCH." >&2
    exit 1
fi

LLVM_ID="${LLVM_REF%%/*}"
LLVM_NAME="${LLVM_ID##*.}"
LLVM_PATH="/usr/lib/sdk/$LLVM_NAME"

echo "OBS runtime:    $OBS_RUNTIME"
echo "Build SDK:      $SDK_REF"
echo "Rust extension: $RUST_REF"
echo "LLVM extension: $LLVM_REF"

flatpak_require "$SDK_REF"
flatpak_require "$RUST_REF"
flatpak_require "$LLVM_REF"

echo "Building EmojiKanban..."

flatpak run \
    --command=bash \
    --filesystem="$PWD" \
    --cwd="$PWD" \
    --share=network \
    --env="FLATPAK_ENABLE_SDK_EXT=rust-nightly,$LLVM_NAME" \
    --env="LIBCLANG_PATH=$LLVM_PATH/lib" \
    "$SDK_REF" \
    -c "source /usr/lib/sdk/rust-nightly/enable.sh &&
        source $LLVM_PATH/enable.sh &&
        ./lin_build.sh"
