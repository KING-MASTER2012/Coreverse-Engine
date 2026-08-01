#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/logger.sh"

TOOL_NAME="CMake Configure"
SOURCE_DIR="."
BUILD_DIR="build"
VCPKG_ROOT="third_party/vcpkg"
MANIFEST_DIR="."
INSTALLED_DIR=""
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --source-dir) SOURCE_DIR="$2"; shift 2 ;;
        --build-dir) BUILD_DIR="$2"; shift 2 ;;
        --vcpkg-root) VCPKG_ROOT="$2"; shift 2 ;;
        --manifest-dir) MANIFEST_DIR="$2"; shift 2 ;;
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

export PATH="$PATH:$HOME/.local/coreverse-bootstrap/cmake/bin:$HOME/.local/coreverse-bootstrap/ninja"

CMAKE_ARGS=(-S "$SOURCE_DIR" -B "$BUILD_DIR")

TOOLCHAIN_FILE="$VCPKG_ROOT/scripts/buildsystems/vcpkg.cmake"
if [ -f "$TOOLCHAIN_FILE" ]; then
    log_info "vcpkg toolchain file found, adding it to CMake." "$TOOL_NAME"
    CMAKE_ARGS+=("-DCMAKE_TOOLCHAIN_FILE=$TOOLCHAIN_FILE")

    if [ -n "$INSTALLED_DIR" ]; then
        # vcpkgInstalledDir from project-paths.json - tells vcpkg's CMake
        # integration where the installed package tree lives, matching the
        # --x-install-root used in the manual 'vcpkg install' step.
        CMAKE_ARGS+=("-DVCPKG_INSTALLED_DIR=$MANIFEST_DIR/$INSTALLED_DIR")
    fi
fi

if [ "$DRY_RUN" = "true" ]; then
    log_info "[DryRun] Would run: cmake ${CMAKE_ARGS[*]}" "$TOOL_NAME"
    write_result "DryRun"
    exit 0
fi

if ! command -v cmake >/dev/null 2>&1; then
    log_error "cmake not found on PATH. The toolchain phase must complete first." "$TOOL_NAME"
    write_result "Failed"
    exit 0
fi

log_info "Running: cmake ${CMAKE_ARGS[*]}" "$TOOL_NAME"
cmake "${CMAKE_ARGS[@]}"

if [ $? -eq 0 ]; then
    log_success "CMake configuration completed ($BUILD_DIR)." "$TOOL_NAME"
    write_result "OK"
else
    log_error "cmake configure failed." "$TOOL_NAME"
    write_result "Failed"
fi
