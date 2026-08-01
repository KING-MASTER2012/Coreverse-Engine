#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/logger.sh"

TOOL_NAME="npm Deps"
PROJECTS="launcher"
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --projects) PROJECTS="$2"; shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        *) shift ;;
    esac
done

RESULT_LINES=""

# Comma-joined list arrives through task-arg word-splitting safely; decode it here.
PROJECTS="${PROJECTS//,/ }"

record() {
    local proj="$1" status="$2"
    RESULT_LINES="${RESULT_LINES}TOOL=npm Deps ($proj)
STATUS=$status
VERSION=

"
}

export PATH="$PATH:$HOME/.local/coreverse-bootstrap/node/bin"

for proj in $PROJECTS; do
    if [ ! -f "$proj/package.json" ]; then
        log_warning "package.json not found ($proj/package.json), skipping." "$TOOL_NAME"
        record "$proj" "Skipped"
        continue
    fi

    if [ "$DRY_RUN" = "true" ]; then
        log_info "[DryRun] Would run npm install in '$proj'." "$TOOL_NAME"
        record "$proj" "DryRun"
        continue
    fi

    if ! command -v npm >/dev/null 2>&1; then
        log_error "npm not found on PATH. The toolchain phase must complete first." "$TOOL_NAME"
        record "$proj" "Failed"
        continue
    fi

    (
        cd "$proj" || exit 1
        if [ -f "package-lock.json" ]; then
            log_info "Running 'npm ci' in '$proj'..." "$TOOL_NAME"
            npm ci
        else
            log_info "Running 'npm install' in '$proj'..." "$TOOL_NAME"
            npm install
        fi
    )

    if [ $? -eq 0 ]; then
        log_success "Dependencies installed for '$proj'." "$TOOL_NAME"
        record "$proj" "OK"
    else
        log_error "npm install failed for '$proj'." "$TOOL_NAME"
        record "$proj" "Failed"
    fi
done

if [ -n "$RESULT_FILE" ]; then
    mkdir -p "$(dirname "$RESULT_FILE")"
    printf '%s' "$RESULT_LINES" > "$RESULT_FILE"
fi
