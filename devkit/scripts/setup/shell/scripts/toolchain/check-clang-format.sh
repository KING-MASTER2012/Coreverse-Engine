#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

# NOTE: this task depends on 'LLVM' in the task graph (DependsOn = LLVM).
# See check-clang-tidy.sh for the shared rationale. MSVC has no native
# formatter, so clang-format is the formatter on all three platforms.

TOOL_NAME="clang-format"
REQUIRED_VERSION=$(read_config_min_version clangFormat)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="22.1.8"
PKG_NAME=$(read_config_pkg_name clangFormat)
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
    local llvm_local_bin="$HOME/.local/coreverse-bootstrap/llvm/bin"
    export PATH="$PATH:$llvm_local_bin"
    command -v clang-format >/dev/null 2>&1 && clang-format --version 2>/dev/null | head -n1
}

upstream_install() {
    local install_dir="$HOME/.local/coreverse-bootstrap/llvm"
    if [ -x "$install_dir/bin/clang-format" ]; then
        export PATH="$PATH:$install_dir/bin"
        return 0
    fi
    log_error "clang-format not found. Re-run check-llvm.sh (or repair the LLVM install manually) - clang-format ships as part of the same LLVM release." "$TOOL_NAME"
    return 1
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
