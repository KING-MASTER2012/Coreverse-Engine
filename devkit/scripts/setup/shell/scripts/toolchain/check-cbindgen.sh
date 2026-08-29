#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

# NOTE: this task depends on 'Cargo' in the task graph (DependsOn = Cargo).
# Must be >= 0.28 - earlier versions cannot parse the `#[unsafe(no_mangle)]`
# attribute syntax the 2024 edition requires (see cmake/FfiHeader.cmake and
# engine/rust/crates/ffi/src/lib.rs). Most distro package managers still ship
# older, which is why this is a cargo-installed tool rather than a
# pacman/apt/dnf/zypper/brew package.

TOOL_NAME="cbindgen"
REQUIRED_VERSION=$(read_config_min_version cbindgen)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="0.28.0"
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
    export PATH="$PATH:$HOME/.cargo/bin"
    command -v cbindgen >/dev/null 2>&1 && cbindgen --version 2>/dev/null | head -n1
}

upstream_install() {
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Cargo not found." "$TOOL_NAME"
        return 1
    fi
    cargo install --locked cbindgen
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
