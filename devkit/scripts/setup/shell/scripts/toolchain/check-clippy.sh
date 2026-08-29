#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

# NOTE: this task depends on 'Rustup' in the task graph (DependsOn = Rustup).
# Clippy is a rustup *component*, not a package-manager or upstream-installer
# tool - installed via `rustup component add clippy`.

TOOL_NAME="Clippy"
REQUIRED_VERSION=$(read_config_min_version clippy)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="0.1.0"
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
    command -v cargo >/dev/null 2>&1 && cargo clippy --version 2>/dev/null | head -n1
}

upstream_install() {
    if ! command -v rustup >/dev/null 2>&1; then
        log_error "Rustup not found." "$TOOL_NAME"
        return 1
    fi
    rustup component add clippy
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
