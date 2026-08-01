#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

TOOL_NAME="Ninja"
REQUIRED_VERSION=$(read_config_min_version ninja)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="1.13.1"
PKG_NAME=$(read_config_pkg_name ninja)
[ -z "$PKG_NAME" ] && PKG_NAME="ninja"
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
    export PATH="$PATH:$HOME/.local/coreverse-bootstrap/ninja"
    command -v ninja >/dev/null 2>&1 && ninja --version 2>/dev/null | head -n1
}

upstream_install() {
    local asset_name
    case "$OS_PLATFORM" in
        macos) asset_name="ninja-mac.zip" ;;
        linux)
            if [ "$OS_ARCH" = "arm64" ]; then
                asset_name="ninja-linux-aarch64.zip"
            else
                asset_name="ninja-linux.zip"
            fi
            ;;
        *) return 1 ;;
    esac

    local api_url="https://api.github.com/repos/ninja-build/ninja/releases/latest"
    local asset_url
    asset_url=$(curl -fsSL "$api_url" \
        | jq -r --arg name "$asset_name" '.assets[] | select(.name == $name) | .browser_download_url' \
        | head -n1)

    if [ -z "$asset_url" ]; then
        log_error "Could not find asset $asset_name in the latest ninja release." "$TOOL_NAME"
        return 1
    fi

    local install_dir="$HOME/.local/coreverse-bootstrap/ninja"
    mkdir -p "$install_dir"
    local archive="/tmp/ninja-release.zip"
    curl -fsSL "$asset_url" -o "$archive"
    unzip -o -q "$archive" -d "$install_dir"
    chmod +x "$install_dir/ninja"

    export PATH="$PATH:$install_dir"
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
