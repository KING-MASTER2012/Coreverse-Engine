#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

# NOTE: this task depends on 'LLVM' in the task graph (DependsOn = LLVM).
# clang-tidy ships in the same LLVM release as clang (a distinct apt/dnf/zypper
# package on some distros, bundled into the "clang" package on Arch/pacman) —
# see tool-versions.json's clangTidy.pkg map. The package-manager step below
# (inherited from invoke_tool_check) is usually enough; the upstream fallback
# only covers the case where check-llvm.sh already pulled a prebuilt tarball.

TOOL_NAME="clang-tidy"
REQUIRED_VERSION=$(read_config_min_version clangTidy)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="22.1.8"
PKG_NAME=$(read_config_pkg_name clangTidy)
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
    command -v clang-tidy >/dev/null 2>&1 && clang-tidy --version 2>/dev/null | sed -n '2p'
}

upstream_install() {
    local install_dir="$HOME/.local/coreverse-bootstrap/llvm"
    if [ -x "$install_dir/bin/clang-tidy" ]; then
        export PATH="$PATH:$install_dir/bin"
        return 0
    fi
    log_error "clang-tidy not found. Re-run check-llvm.sh (or repair the LLVM install manually) - clang-tidy ships as part of the same LLVM release." "$TOOL_NAME"
    return 1
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
