#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

# Confirmed as its own tool (not replaced by GCC's built-in -fanalyzer flag,
# which needs no separate install). cppcheck runs independently of which
# compiler is active, so it's checked on Linux and macOS alike.

TOOL_NAME="cppcheck"
REQUIRED_VERSION=$(read_config_min_version cppcheck)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="2.13.0"
PKG_NAME=$(read_config_pkg_name cppcheck)
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
    command -v cppcheck >/dev/null 2>&1 && cppcheck --version 2>/dev/null | head -n1
}

upstream_install() {
    log_error "No upstream installer for cppcheck - it must come from the package manager ($PKG_MANAGER)." "$TOOL_NAME"
    return 1
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
