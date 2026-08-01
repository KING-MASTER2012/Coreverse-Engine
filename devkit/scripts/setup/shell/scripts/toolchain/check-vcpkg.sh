#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/logger.sh"

# NOTE: this task depends on 'Git' in the task graph (DependsOn = Git).
# vcpkg is installed project-locally (not system-wide) and is identical on
# Linux/macOS: clone + run bootstrap-vcpkg.sh.
# Matches project-paths.json's "vcpkgRoot" key (renamed from "vcpkgInstallDir").

TOOL_NAME="vcpkg"
VCPKG_ROOT="third_party/vcpkg"
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --vcpkg-root) VCPKG_ROOT="$2"; shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        --required-version) shift 2 ;;  # accepted for CLI consistency, unused
        *) shift ;;
    esac
done

write_result() {
    local file="$1" tool="$2" status="$3" version="$4"
    if [ -z "$file" ]; then
        return 0
    fi
    mkdir -p "$(dirname "$file")"
    printf 'TOOL=%s\nSTATUS=%s\nVERSION=%s\n' "$tool" "$status" "$version" > "$file"
}

if ! command -v git >/dev/null 2>&1; then
    log_error "Git not found. vcpkg needs Git to clone." "$TOOL_NAME"
    write_result "$RESULT_FILE" "$TOOL_NAME" "Failed" ""
    exit 0
fi

VCPKG_BIN="$VCPKG_ROOT/vcpkg"
ALREADY_CLONED="false"
[ -x "$VCPKG_BIN" ] && ALREADY_CLONED="true"

if [ "$ALREADY_CLONED" = "true" ]; then
    PREVIOUS_VERSION=$("$VCPKG_BIN" version 2>/dev/null | head -n1)
    log_info "Found: $VCPKG_ROOT ($PREVIOUS_VERSION)" "$TOOL_NAME"
else
    log_warning "$VCPKG_ROOT not found, will perform the initial setup." "$TOOL_NAME"
fi

if [ "$DRY_RUN" = "true" ]; then
    if [ "$ALREADY_CLONED" = "true" ]; then
        log_info "[DryRun] Would run: git pull + re-bootstrap." "$TOOL_NAME"
    else
        log_info "[DryRun] Would run: clone + bootstrap." "$TOOL_NAME"
    fi
    write_result "$RESULT_FILE" "$TOOL_NAME" "DryRun" ""
    exit 0
fi

if [ "$ALREADY_CLONED" = "true" ]; then
    log_info "Updating existing vcpkg checkout (git pull)..." "$TOOL_NAME"
    (cd "$VCPKG_ROOT" && git pull --ff-only) >/dev/null 2>&1
else
    PARENT_DIR=$(dirname "$VCPKG_ROOT")
    [ -n "$PARENT_DIR" ] && [ "$PARENT_DIR" != "." ] && mkdir -p "$PARENT_DIR"
    log_info "Cloning https://github.com/microsoft/vcpkg.git ..." "$TOOL_NAME"
    git clone --depth 1 https://github.com/microsoft/vcpkg.git "$VCPKG_ROOT" >/dev/null 2>&1
fi

log_info "Running bootstrap-vcpkg.sh..." "$TOOL_NAME"
( cd "$VCPKG_ROOT" && ./bootstrap-vcpkg.sh -disableMetrics ) >/dev/null 2>&1

if [ -x "$VCPKG_BIN" ]; then
    FINAL_VERSION=$("$VCPKG_BIN" version 2>/dev/null | head -n1)
    if [ "$ALREADY_CLONED" = "true" ]; then
        log_success "Ready (updated): $FINAL_VERSION" "$TOOL_NAME"
        write_result "$RESULT_FILE" "$TOOL_NAME" "Upgraded" "$FINAL_VERSION"
    else
        log_success "Ready (installed): $FINAL_VERSION" "$TOOL_NAME"
        write_result "$RESULT_FILE" "$TOOL_NAME" "Installed" "$FINAL_VERSION"
    fi
else
    log_error "vcpkg binary not found after bootstrap." "$TOOL_NAME"
    write_result "$RESULT_FILE" "$TOOL_NAME" "Failed" ""
fi
