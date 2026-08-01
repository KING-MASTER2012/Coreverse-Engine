#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

TOOL_NAME="Rustup"
REQUIRED_VERSION=$(read_config_min_version rustup)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="1.29.0"
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
    command -v rustup >/dev/null 2>&1 && rustup --version 2>/dev/null | head -n1
}

upstream_install() {
    # rustup's official installer script works identically on Linux and macOS.
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default

    local cargo_bin="$HOME/.cargo/bin"
    if [ -d "$cargo_bin" ]; then
        export PATH="$PATH:$cargo_bin"
        add_to_shell_profile "$cargo_bin"
    fi
}

# add_to_shell_profile <dir>
# Best-effort: appends a PATH export to the user's shell profile so the tool
# is still available in new terminal sessions after bootstrap finishes.
add_to_shell_profile() {
    local dir="$1"
    local profile="$HOME/.profile"
    [ -n "${ZSH_VERSION:-}" ] && profile="$HOME/.zshrc"
    [ -n "${BASH_VERSION:-}" ] && [ -f "$HOME/.bashrc" ] && profile="$HOME/.bashrc"

    if [ -f "$profile" ] && grep -qF "$dir" "$profile" 2>/dev/null; then
        return 0
    fi
    printf '\nexport PATH="$PATH:%s"\n' "$dir" >> "$profile" 2>/dev/null || true
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
