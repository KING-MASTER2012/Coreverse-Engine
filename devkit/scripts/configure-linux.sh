#!/usr/bin/env bash
# Coreverse - Linux configure wrapper with an architecture sanity check.
#
# Lower-probability sibling of configure-windows.ps1 / configure-macos.sh:
# Linux has no per-shell host/target tool selection the way MSVC does, so the
# x86-vs-x64 class of bug this whole Faz 8.4 batch targets is much rarer here.
# What CAN still happen: a container/chroot/multiarch setup where the running
# shell's `uname -m` doesn't match the architecture CMakePresets.json actually
# targets (currently only x64-linux - see linux-x64-gcc/linux-x64-clang).
# This is a defensive check, not a fix for a confirmed bug report.
#
# Usage:
#   ./devkit/scripts/configure-linux.sh [--preset <name>] [-- <extra cmake args>]
# Defaults to the linux-x64-gcc preset (COMPILER_ROLE: PRIMARY in CMakePresets.json).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/setup/shell/scripts/common/logger.sh"

log_banner "Coreverse - Linux Configure"

PRESET="linux-x64-gcc"
EXTRA_ARGS=()

while [ $# -gt 0 ]; do
    case "$1" in
        --preset) PRESET="$2"; shift 2 ;;
        --) shift; EXTRA_ARGS+=("$@"); break ;;
        *) EXTRA_ARGS+=("$1"); shift ;;
    esac
done

# --- 1. Architecture sanity check ---
# All current Linux presets target x64-linux; there is no arm64-linux preset
# yet. If the host isn't x86_64, cmake --preset would still "succeed" but
# vcpkg would be building/fetching the wrong triplet's packages entirely.
HOST_ARCH="$(uname -m)"

case "$HOST_ARCH" in
    x86_64|amd64)
        log_success "Host architecture: $HOST_ARCH (matches the x64-linux triplet all current presets use)."
        ;;
    *)
        log_warning "Host architecture is '$HOST_ARCH', but every Linux preset in CMakePresets.json targets x64-linux."
        log_warning "This looks like a container/chroot/multiarch environment, or a non-x86_64 host - no matching preset exists yet."
        log_warning "Continuing anyway per this script's defensive (non-blocking) policy; expect vcpkg/link errors if this isn't actually x86_64 underneath."
        ;;
esac

# --- 2. Forward to cmake --preset ---
log_info "Running: cmake --preset $PRESET ${EXTRA_ARGS[*]:-}"

cd "$REPO_ROOT" || exit 1
if cmake --preset "$PRESET" "${EXTRA_ARGS[@]}"; then
    log_success "CMake configuration completed successfully."
else
    log_error "cmake --preset $PRESET failed (exit code $?)."
    exit 1
fi
