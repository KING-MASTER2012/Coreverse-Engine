#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

TOOL_NAME="Go"
REQUIRED_VERSION=$(read_config_min_version go)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="1.26.5"
PKG_NAME=$(read_config_pkg_name go)
[ -z "$PKG_NAME" ] && PKG_NAME="go"
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
    export PATH="$PATH:/usr/local/go/bin:$HOME/.local/coreverse-bootstrap/go/bin"
    command -v go >/dev/null 2>&1 && go version 2>/dev/null
}

upstream_install() {
    local go_os
    case "$OS_PLATFORM" in
        macos) go_os="darwin" ;;
        linux) go_os="linux" ;;
        *) return 1 ;;
    esac

    # go.dev publishes a JSON index of all releases; take the first stable one
    # for our OS/arch (same API used by the Windows/PowerShell phase).
    local filename
    filename=$(curl -fsSL "https://go.dev/dl/?mode=json" \
        | jq -r --arg os "$go_os" --arg arch "$OS_ARCH" \
            '[.[] | select(.stable == true)][0].files[] | select(.os == $os and .arch == $arch and .kind == "archive") | .filename' \
        | head -n1)

    if [ -z "$filename" ]; then
        log_error "Could not find a stable Go release for $go_os/$OS_ARCH." "$TOOL_NAME"
        return 1
    fi

    local install_dir="$HOME/.local/coreverse-bootstrap/go"
    local archive="/tmp/$filename"
    curl -fsSL "https://go.dev/dl/$filename" -o "$archive"

    rm -rf "$install_dir"
    mkdir -p "$(dirname "$install_dir")"
    tar -xf "$archive" -C "$(dirname "$install_dir")"
    mv "$(dirname "$install_dir")/go" "$install_dir"

    export PATH="$PATH:$install_dir/bin"
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
