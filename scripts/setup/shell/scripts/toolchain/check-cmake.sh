#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

TOOL_NAME="CMake"
REQUIRED_VERSION=$(read_config_min_version cmake)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="4.4.0"
PKG_NAME=$(read_config_pkg_name cmake)
[ -z "$PKG_NAME" ] && PKG_NAME="cmake"
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --required-version) REQUIRED_VERSION="$2"; shift 2 ;;
        --pkg-name) PKG_NAME="$2"; shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        *) shift ;;
    esac
done

get_version_raw() {
    export PATH="$PATH:$HOME/.local/coreverse-bootstrap/cmake/bin"
    command -v cmake >/dev/null 2>&1 && cmake --version 2>/dev/null | head -n1
}

upstream_install() {
    # Kitware publishes official prebuilt archives for both Linux and macOS.
    local os_pattern
    case "$OS_PLATFORM" in
        macos) os_pattern="macos-universal" ;;
        linux) os_pattern="linux-${OS_ARCH}" ;;
        *) return 1 ;;
    esac

    local api_url="https://api.github.com/repos/Kitware/CMake/releases/latest"
    local asset_url
    asset_url=$(curl -fsSL "$api_url" \
        | jq -r --arg pat "${os_pattern}\\.tar\\.gz$" '.assets[] | select(.name | test($pat)) | .browser_download_url' \
        | head -n1)

    if [ -z "$asset_url" ]; then
        log_error "Could not find a matching prebuilt CMake release for $OS_PLATFORM/$OS_ARCH." "$TOOL_NAME"
        return 1
    fi

    local install_dir="$HOME/.local/coreverse-bootstrap/cmake"
    mkdir -p "$install_dir"
    local archive="/tmp/cmake-release.tar.gz"
    curl -fsSL "$asset_url" -o "$archive"

    if [ "$OS_PLATFORM" = "macos" ]; then
        # macOS archives contain CMake.app; the CLI binaries live under Contents/bin.
        tar -xf "$archive" -C "$install_dir" --strip-components=1
        if [ -d "$install_dir/CMake.app/Contents/bin" ]; then
            ln -sf "$install_dir/CMake.app/Contents/bin"/* "$install_dir/bin/" 2>/dev/null || true
            mkdir -p "$install_dir/bin"
            for f in "$install_dir/CMake.app/Contents/bin"/*; do
                ln -sf "$f" "$install_dir/bin/$(basename "$f")"
            done
        fi
    else
        tar -xf "$archive" -C "$install_dir" --strip-components=1
    fi

    export PATH="$PATH:$install_dir/bin"
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
