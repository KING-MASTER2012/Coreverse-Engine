#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/logger.sh"

TOOL_NAME="Go Deps"
MODULES="server"
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --modules) MODULES="$2"; shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        *) shift ;;
    esac
done

RESULT_LINES=""

# Comma-joined list arrives through task-arg word-splitting safely; decode it here.
MODULES="${MODULES//,/ }"

record() {
    local mod="$1" status="$2"
    RESULT_LINES="${RESULT_LINES}TOOL=Go Deps ($mod)
STATUS=$status
VERSION=

"
}

export PATH="$PATH:/usr/local/go/bin:$HOME/.local/coreverse-bootstrap/go/bin"

for mod in $MODULES; do
    if [ ! -f "$mod/go.mod" ]; then
        log_warning "go.mod not found ($mod/go.mod), skipping." "$TOOL_NAME"
        record "$mod" "Skipped"
        continue
    fi

    if [ "$DRY_RUN" = "true" ]; then
        log_info "[DryRun] Would run 'go mod download' in '$mod'." "$TOOL_NAME"
        record "$mod" "DryRun"
        continue
    fi

    if ! command -v go >/dev/null 2>&1; then
        log_error "go not found on PATH. The toolchain phase must complete first." "$TOOL_NAME"
        record "$mod" "Failed"
        continue
    fi

    (
        cd "$mod" || exit 1
        log_info "Running 'go mod download' in '$mod'..." "$TOOL_NAME"
        go mod download
    )

    if [ $? -eq 0 ]; then
        log_success "Dependencies downloaded for '$mod'." "$TOOL_NAME"
        record "$mod" "OK"
    else
        log_error "go mod download failed for '$mod'." "$TOOL_NAME"
        record "$mod" "Failed"
    fi
done

if [ -n "$RESULT_FILE" ]; then
    mkdir -p "$(dirname "$RESULT_FILE")"
    printf '%s' "$RESULT_LINES" > "$RESULT_FILE"
fi
