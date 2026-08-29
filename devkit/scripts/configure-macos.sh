#!/usr/bin/env bash
# Coreverse - macOS configure wrapper with native-arch enforcement.
#
# Same bug class as configure-windows.ps1's x86-vs-x64 problem, transposed to
# Apple Silicon: a terminal running under Rosetta 2 reports itself as x86_64
# even on an arm64 Mac, which would make cmake/vcpkg build against the wrong
# triplet while CMakePresets.json's only macOS preset (macos-arm64-clang)
# hardcodes VCPKG_TARGET_TRIPLET=arm64-osx - guaranteeing the same class of
# "host tools don't match target lib architecture" link failure.
#
# This script detects Rosetta translation via `sysctl sysctl.proc_translated`
# and, if translated, re-execs itself natively under `arch -arm64` before
# doing anything else - so the whole rest of the configure step (and cmake
# itself) always runs as genuine arm64, regardless of which Terminal profile
# or shell launched it.
#
# Usage:
#   ./devkit/scripts/configure-macos.sh [--preset <name>] [-- <extra cmake args>]
# Defaults to the macos-arm64-clang preset (the only macOS preset defined).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/setup/shell/scripts/common/logger.sh"

log_banner "Coreverse - macOS Configure"

# --- 1. Rosetta detection + native re-exec ---
# sysctl.proc_translated: 1 if this process is running under Rosetta 2
# translation, 0 if native, missing entirely on Intel Macs (treated as "0").
IS_TRANSLATED="$(sysctl -in sysctl.proc_translated 2>/dev/null || echo 0)"
HOST_ARCH="$(uname -m)"

if [ "$IS_TRANSLATED" = "1" ]; then
    if [ -z "${CV_MACOS_REEXEC:-}" ]; then
        log_warning "This shell is running under Rosetta 2 (translated x86_64) on an Apple Silicon Mac."
        log_warning "Re-executing natively under arm64 so cmake/vcpkg don't end up building against the wrong triplet..."
        export CV_MACOS_REEXEC=1
        exec arch -arm64 "$0" "$@"
    fi
    # If we get here, the re-exec itself didn't take effect - fail loudly
    # rather than silently continuing under the wrong architecture.
    log_error "Re-exec into arm64 did not take effect (still translated). Try running this from Terminal.app directly instead of an x86_64-pinned terminal profile."
    exit 1
elif [ "$HOST_ARCH" != "arm64" ]; then
    log_warning "Host architecture is '$HOST_ARCH' (a genuine Intel Mac, not Rosetta translation)."
    log_warning "CMakePresets.json currently only defines macos-arm64-clang (VCPKG_TARGET_TRIPLET=arm64-osx) - there is no x64-osx preset yet."
    log_warning "Continuing anyway per this script's defensive (non-blocking) policy, but cmake --preset will likely fail without an Intel preset."
else
    log_success "Native arm64 confirmed (no Rosetta translation)."
fi

# --- 2. Forward to cmake --preset ---
PRESET="macos-arm64-clang"
EXTRA_ARGS=()

while [ $# -gt 0 ]; do
    case "$1" in
        --preset) PRESET="$2"; shift 2 ;;
        --) shift; EXTRA_ARGS+=("$@"); break ;;
        *) EXTRA_ARGS+=("$1"); shift ;;
    esac
done

log_info "Running: cmake --preset $PRESET ${EXTRA_ARGS[*]:-}"

cd "$REPO_ROOT" || exit 1
if cmake --preset "$PRESET" "${EXTRA_ARGS[@]}"; then
    log_success "CMake configuration completed successfully."
else
    log_error "cmake --preset $PRESET failed (exit code $?)."
    exit 1
fi
