#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

TOOL_NAME="Node.js"
REQUIRED_VERSION=$(read_config_min_version node)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="24.18.0"
PKG_NAME=$(read_config_pkg_name node)
[ -z "$PKG_NAME" ] && PKG_NAME="nodejs npm"
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
    export PATH="$PATH:$HOME/.local/coreverse-bootstrap/node/bin"
    command -v node >/dev/null 2>&1 && node --version 2>/dev/null
}

upstream_install() {
    local node_os
    case "$OS_PLATFORM" in
        macos) node_os="darwin" ;;
        linux) node_os="linux" ;;
        *) return 1 ;;
    esac

    # nodejs.org publishes an index of all releases; take the first LTS entry
    # (same API used by the Windows/PowerShell phase).
    local version
    version=$(curl -fsSL "https://nodejs.org/dist/index.json" | jq -r '[.[] | select(.lts != false)][0].version')

    if [ -z "$version" ]; then
        log_error "Could not determine the latest Node.js LTS version." "$TOOL_NAME"
        return 1
    fi

    local filename="node-${version}-${node_os}-${OS_ARCH}.tar.gz"
    local install_dir="$HOME/.local/coreverse-bootstrap/node"
    local archive="/tmp/$filename"

    curl -fsSL "https://nodejs.org/dist/${version}/${filename}" -o "$archive"
    if [ ! -s "$archive" ]; then
        log_error "Failed to download $filename." "$TOOL_NAME"
        return 1
    fi

    rm -rf "$install_dir"
    mkdir -p "$(dirname "$install_dir")"
    tar -xf "$archive" -C "$(dirname "$install_dir")"
    mv "$(dirname "$install_dir")/node-${version}-${node_os}-${OS_ARCH}" "$install_dir"

    export PATH="$PATH:$install_dir/bin"
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
