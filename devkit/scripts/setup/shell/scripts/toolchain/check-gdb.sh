#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

# GDB is usually preinstalled on Linux desktop distros but NOT guaranteed on
# minimal images (some Arch/container base images ship without it) - hence a
# real check instead of assuming it's there. On macOS it's not preinstalled at
# all (Apple pushes lldb instead - see check-lldb.sh); still checked here for
# completeness since it's installable via Homebrew.

TOOL_NAME="GDB"
REQUIRED_VERSION=$(read_config_min_version gdb)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="12.0.0"
PKG_NAME=$(read_config_pkg_name gdb)
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --required-version) REQUIRED_VERSION="$2"; shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        *) shift ;;
    esac
done

get_version_raw() {
    command -v gdb >/dev/null 2>&1 && gdb --version 2>/dev/null | head -n1
}

upstream_install() {
    log_error "No upstream installer for GDB - it must come from the package manager ($PKG_MANAGER)." "$TOOL_NAME"
    return 1
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
