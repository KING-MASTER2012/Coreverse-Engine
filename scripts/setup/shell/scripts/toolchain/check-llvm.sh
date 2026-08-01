#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

TOOL_NAME="LLVM/Clang"
REQUIRED_VERSION=$(read_config_min_version llvm)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="22.1.8"
PKG_NAME=$(read_config_pkg_name llvm)
[ -z "$PKG_NAME" ] && PKG_NAME="clang llvm"
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
    export PATH="$PATH:$HOME/.local/coreverse-bootstrap/llvm/bin"
    command -v clang >/dev/null 2>&1 && clang --version 2>/dev/null | head -n1
}

upstream_install() {
    if [ "$OS_PLATFORM" = "linux" ] && { [ "$OS_DISTRO" = "debian" ] || [ "$OS_DISTRO" = "kali" ]; }; then
        # apt.llvm.org's official bootstrap script always installs the latest release.
        log_info "Using the official apt.llvm.org bootstrap script..." "$TOOL_NAME"
        curl -fsSL https://apt.llvm.org/llvm.sh -o /tmp/llvm.sh
        chmod +x /tmp/llvm.sh
        with_pkg_lock "sudo /tmp/llvm.sh"
        return $?
    fi

    # Generic fallback: download a prebuilt release from GitHub and place it
    # under a user-local directory (no root required).
    local api_url="https://api.github.com/repos/llvm/llvm-project/releases/latest"
    local os_pattern
    case "$OS_PLATFORM" in
        macos) os_pattern="apple-darwin" ;;
        linux) os_pattern="linux-gnu" ;;
        *) return 1 ;;
    esac

    local asset_url
    asset_url=$(curl -fsSL "$api_url" \
        | jq -r --arg pat "${OS_ARCH}.*${os_pattern}.*\\.tar\\.[gx]z$" '.assets[] | select(.name | test($pat)) | .browser_download_url' \
        | head -n1)

    if [ -z "$asset_url" ]; then
        log_error "Could not find a matching prebuilt LLVM release for $OS_ARCH/$OS_PLATFORM." "$TOOL_NAME"
        return 1
    fi

    local install_dir="$HOME/.local/coreverse-bootstrap/llvm"
    mkdir -p "$install_dir"
    local archive="/tmp/llvm-release.tar.xz"
    curl -fsSL "$asset_url" -o "$archive"
    tar -xf "$archive" -C "$install_dir" --strip-components=1

    export PATH="$PATH:$install_dir/bin"
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
