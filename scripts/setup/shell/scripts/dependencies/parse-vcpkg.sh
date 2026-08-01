#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/logger.sh"

TOOL_NAME="vcpkg Deps"
MANIFEST_DIR="."
VCPKG_ROOT="third_party/vcpkg"
INSTALLED_DIR=""
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --manifest-dir) MANIFEST_DIR="$2"; shift 2 ;;
        --vcpkg-root) VCPKG_ROOT="$2"; shift 2 ;;
        --installed-dir) INSTALLED_DIR="$2"; shift 2 ;;
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

if [ ! -f "$MANIFEST_DIR/vcpkg.json" ]; then
    log_warning "vcpkg.json not found ($MANIFEST_DIR/vcpkg.json), skipping." "$TOOL_NAME"
    write_result "Skipped"
    exit 0
fi

VCPKG_INSTALL_ARGS=("--x-manifest-root=$MANIFEST_DIR")
if [ -n "$INSTALLED_DIR" ]; then
    # vcpkgInstalledDir from project-paths.json - kept relative to the manifest
    # dir, matching vcpkg's own default layout convention.
    VCPKG_INSTALL_ARGS+=("--x-install-root=$MANIFEST_DIR/$INSTALLED_DIR")
fi

if [ "$DRY_RUN" = "true" ]; then
    log_info "[DryRun] Would run: vcpkg install ${VCPKG_INSTALL_ARGS[*]}" "$TOOL_NAME"
    write_result "DryRun"
    exit 0
fi

VCPKG_BIN="$VCPKG_ROOT/vcpkg"
if [ ! -x "$VCPKG_BIN" ]; then
    log_error "vcpkg binary not found ($VCPKG_BIN). The toolchain phase (check-vcpkg) must complete first." "$TOOL_NAME"
    write_result "Failed"
    exit 0
fi

log_info "Running vcpkg install (manifest mode)..." "$TOOL_NAME"
"$VCPKG_BIN" install "${VCPKG_INSTALL_ARGS[@]}"

if [ $? -eq 0 ]; then
    log_success "C++ dependencies (vcpkg) installed." "$TOOL_NAME"
    write_result "OK"
else
    log_error "vcpkg install failed." "$TOOL_NAME"
    write_result "Failed"
fi
