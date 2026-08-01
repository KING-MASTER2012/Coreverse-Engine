#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/logger.sh"

TOOL_NAME="Cargo Deps"
WORKSPACE_ROOT="."
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --workspace-root) WORKSPACE_ROOT="$2"; shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        *) shift ;;
    esac
done

write_result() {
    local status="$1"
    [ -z "$RESULT_FILE" ] && return 0
    mkdir -p "$(dirname "$RESULT_FILE")"
    printf 'TOOL=%s\nSTATUS=%s\nVERSION=%s\n' "$TOOL_NAME" "$status" "" > "$RESULT_FILE"
}

export PATH="$PATH:$HOME/.cargo/bin"

if [ ! -f "$WORKSPACE_ROOT/Cargo.toml" ]; then
    log_warning "Cargo.toml not found ($WORKSPACE_ROOT/Cargo.toml), skipping." "$TOOL_NAME"
    write_result "Skipped"
    exit 0
fi

if [ "$DRY_RUN" = "true" ]; then
    log_info "[DryRun] Would run 'cargo fetch' in '$WORKSPACE_ROOT'." "$TOOL_NAME"
    write_result "DryRun"
    exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
    log_error "cargo not found on PATH. The toolchain phase must complete first." "$TOOL_NAME"
    write_result "Failed"
    exit 0
fi

(
    cd "$WORKSPACE_ROOT" || exit 1
    if [ -f "Cargo.lock" ]; then
        log_info "Running: cargo fetch --locked" "$TOOL_NAME"
        cargo fetch --locked
    else
        log_info "Running: cargo fetch" "$TOOL_NAME"
        cargo fetch
    fi
)

if [ $? -eq 0 ]; then
    log_success "Rust dependencies fetched." "$TOOL_NAME"
    write_result "OK"
else
    log_error "cargo fetch failed." "$TOOL_NAME"
    write_result "Failed"
fi
