#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

# GCC is the stated primary compiler on Linux (README.md), but tool-versions.json
# had no entry for it until now and bootstrap never verified it - this script
# closes that gap. On macOS, `gcc`/`cc` are aliases Apple points at its own
# Clang, so a real-GCC check doesn't apply there; skip cleanly instead of
# reporting a false failure.

TOOL_NAME="GCC"
REQUIRED_VERSION=$(read_config_min_version gcc)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="12.0.0"
PKG_NAME=$(read_config_pkg_name gcc)
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

if [ "$OS_PLATFORM" = "macos" ]; then
    log_info "Apple ships Clang under the gcc/cc name on macOS; skipping the real-GCC check (see check-llvm.sh for the actual compiler check on this platform)." "$TOOL_NAME"
    write_result "$RESULT_FILE" "$TOOL_NAME" "OK" "skipped-on-macos"
    exit 0
fi

get_version_raw() {
    command -v gcc >/dev/null 2>&1 && gcc --version 2>/dev/null | head -n1
}

upstream_install() {
    log_error "No upstream installer for GCC - it must come from the distro package manager. Verify $PKG_MANAGER can reach its repositories." "$TOOL_NAME"
    return 1
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
