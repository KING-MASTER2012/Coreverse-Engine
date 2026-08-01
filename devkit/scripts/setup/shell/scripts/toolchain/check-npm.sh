#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

# NOTE: this task depends on 'Node.js' in the task graph (DependsOn = Node.js).
# npm ships with Node.js; it has no separate package-manager or upstream install.

TOOL_NAME="npm"
REQUIRED_VERSION=$(read_config_min_version npm)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="11.16.0"
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
    export PATH="$PATH:$HOME/.local/coreverse-bootstrap/node/bin"
    command -v npm >/dev/null 2>&1 && npm --version 2>/dev/null
}

upstream_install() {
    if ! command -v node >/dev/null 2>&1; then
        log_error "Node.js not found. npm ships with Node.js; install Node.js first." "$TOOL_NAME"
        return 1
    fi
    npm install -g npm@latest
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
