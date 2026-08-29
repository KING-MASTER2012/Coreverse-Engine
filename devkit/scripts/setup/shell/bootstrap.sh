#!/usr/bin/env bash
# Coreverse Bootstrap - Linux/macOS entry point.
#
# Automates toolchain detection/installation, project dependency installation,
# and CMake configuration in a single command. Independent tools run in
# parallel; dependent ones (Rustup->Cargo, Git->vcpkg) run in
# dependency-ordered layers. Mirrors the setup/powershell/bootstrap.ps1 design.
#
# Usage:
#   ./bootstrap.sh
#   ./bootstrap.sh --dry-run
#   ./bootstrap.sh --yes
#
# Must be run from the Coreverse Engine repository root (see setup/config/README.md).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_DIR="$SCRIPT_DIR/../config"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/common/logger.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/common/os-detect.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/common/version-compare.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/common/pkg-lock.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/common/pkg-dispatch.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/common/parallel-runner.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/package-managers/pacman.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/package-managers/apt.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/package-managers/dnf.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/package-managers/zypper.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/package-managers/brew.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR/scripts/final/summary-table.sh"

# --- 0. CLI arguments ---
YES="false"
DRY_RUN="false"

for arg in "$@"; do
    case "$arg" in
        --yes|-y) YES="true" ;;
        --dry-run) DRY_RUN="true" ;;
        *) log_warning "Unknown argument: $arg" ;;
    esac
done

DRY_RUN_FLAG=""
[ "$DRY_RUN" = "true" ] && DRY_RUN_FLAG="--dry-run"

log_banner "Coreverse Bootstrap (Linux/macOS)"

# --- 1. Prerequisite: curl must exist (everything else bootstraps from it) ---
if ! command -v curl >/dev/null 2>&1; then
    log_error "curl is required but was not found on PATH."
    log_error "Please install curl manually and re-run this script."
    exit 1
fi

# --- 2. OS / distro detection ---
if ! detect_os; then
    log_error "Unsupported operating system or Linux distribution."
    log_error "Supported: Arch-based, Debian-based, Fedora, openSUSE, RHEL-based, Kali, macOS."
    echo ""
    log_plain "Official installation docs for each required tool:"
    log_plain "  - Rustup:     https://www.rust-lang.org/tools/install"
    log_plain "  - Cargo:      https://www.rust-lang.org/tools/install"
    log_plain "  - Git:        https://git-scm.com/downloads"
    log_plain "  - LLVM/Clang: https://releases.llvm.org/download.html"
    log_plain "  - CMake:      https://cmake.org/download/"
    log_plain "  - Ninja:      https://github.com/ninja-build/ninja/releases"
    log_plain "  - vcpkg:      https://learn.microsoft.com/vcpkg/get_started/overview"
    exit 1
fi

log_info "$OS_PLATFORM ($OS_DISTRO) | $OS_ARCH | package manager: $PKG_MANAGER"

# --- 3. Homebrew bootstrap (macOS only) ---
if [ "$PKG_MANAGER" = "brew" ]; then
    ensure_homebrew_installed || log_warning "Homebrew could not be installed automatically."
fi

# --- 4. sudo access (Linux only - cached once up front, then silent for the rest of the run) ---
SUDO_KEEPALIVE_PID=""

request_sudo_if_needed() {
    [ "$OS_PLATFORM" != "linux" ] && return 0
    if [ "$(id -u)" -eq 0 ]; then
        log_success "Already running as root."
        return 0
    fi
    if ! command -v sudo >/dev/null 2>&1; then
        log_warning "sudo not found; package-manager installs may fail without root privileges."
        return 0
    fi

    log_info "Requesting sudo access once up front (stays cached for the rest of the run)..."
    if ! sudo -v; then
        log_error "Could not obtain sudo access."
        exit 1
    fi

    ( while true; do sudo -n true; sleep 60; kill -0 "$$" 2>/dev/null || exit; done ) &
    SUDO_KEEPALIVE_PID=$!
}

cleanup() {
    [ -n "$SUDO_KEEPALIVE_PID" ] && kill "$SUDO_KEEPALIVE_PID" 2>/dev/null
}
trap cleanup EXIT

request_sudo_if_needed

if [ "$DRY_RUN" = "true" ]; then
    log_warning "DRY-RUN mode active: no installs/changes will be made."
fi

# --- 5. jq (needed to read the shared JSON config) ---
PROJECT_PATHS_JSON="$CONFIG_DIR/project-paths.json"

ENGINE_ROOT="$(cd "$SCRIPT_DIR/../../../../" && pwd)"

to_abs_path() {
    local rel_path="$1"
    if [ -z "$rel_path" ] || [ "$rel_path" = "null" ]; then
        echo ""
    elif [[ "$rel_path" = /* ]]; then
        echo "$rel_path"
    else
        echo "$ENGINE_ROOT/$rel_path"
    fi
}

CARGO_WORKSPACE_ROOT=$(to_abs_path "$(jq -r '.cargoWorkspaceRoot' "$PROJECT_PATHS_JSON")")
VCPKG_MANIFEST_DIR=$(to_abs_path "$(jq -r '.vcpkgManifestDir' "$PROJECT_PATHS_JSON")")
VCPKG_ROOT=$(to_abs_path "$(jq -r '.vcpkgRoot' "$PROJECT_PATHS_JSON")")
VCPKG_INSTALLED_DIR=$(to_abs_path "$(jq -r '.vcpkgInstalledDir // empty' "$PROJECT_PATHS_JSON")")
CMAKE_SOURCE_DIR=$(to_abs_path "$(jq -r '.cmakeSourceDir' "$PROJECT_PATHS_JSON")")
CMAKE_BUILD_DIR=$(to_abs_path "$(jq -r '.cmakeBuildDir' "$PROJECT_PATHS_JSON")")

RESULTS_DIR=$(mktemp -d)

# --- 6. Phase 1/3: toolchain checks (dependency graph, parallel) ---
log_banner "1/3 - Toolchain Check"

TOOLCHAIN_DIR="$SCRIPT_DIR/scripts/toolchain"

TOOLCHAIN_TASKS=(
    "Rustup|$TOOLCHAIN_DIR/check-rustup.sh|$DRY_RUN_FLAG|"
    "Cargo|$TOOLCHAIN_DIR/check-cargo.sh|$DRY_RUN_FLAG|Rustup"
    "Git|$TOOLCHAIN_DIR/check-git.sh|$DRY_RUN_FLAG|"
    "LLVM|$TOOLCHAIN_DIR/check-llvm.sh|$DRY_RUN_FLAG|"
    "CMake|$TOOLCHAIN_DIR/check-cmake.sh|$DRY_RUN_FLAG|"
    "Ninja|$TOOLCHAIN_DIR/check-ninja.sh|$DRY_RUN_FLAG|"
    "vcpkg|$TOOLCHAIN_DIR/check-vcpkg.sh|--vcpkg-root $VCPKG_ROOT $DRY_RUN_FLAG|Git"

    # --- LLVM sub-tools: ship in the same LLVM release as clang, so they only
    #     need the base LLVM install to have landed first. ---
    "clang-tidy|$TOOLCHAIN_DIR/check-clang-tidy.sh|$DRY_RUN_FLAG|LLVM"
    "clang-format|$TOOLCHAIN_DIR/check-clang-format.sh|$DRY_RUN_FLAG|LLVM"
    "LLDB|$TOOLCHAIN_DIR/check-lldb.sh|$DRY_RUN_FLAG|LLVM"

    # --- Rust extras: component-based tools need Rustup; cargo-installed
    #     tools need Cargo. ---
    "Clippy|$TOOLCHAIN_DIR/check-clippy.sh|$DRY_RUN_FLAG|Rustup"
    "rustfmt|$TOOLCHAIN_DIR/check-rustfmt.sh|$DRY_RUN_FLAG|Rustup"
    "mdBook|$TOOLCHAIN_DIR/check-mdbook.sh|$DRY_RUN_FLAG|Cargo"
    "cbindgen|$TOOLCHAIN_DIR/check-cbindgen.sh|$DRY_RUN_FLAG|Cargo"

    # --- Linux toolchain (GCC is the stated primary compiler; skips cleanly
    #     on macOS - see check-gcc.sh). Independent of everything above. ---
    "GCC|$TOOLCHAIN_DIR/check-gcc.sh|$DRY_RUN_FLAG|"
    "GDB|$TOOLCHAIN_DIR/check-gdb.sh|$DRY_RUN_FLAG|"
    "cppcheck|$TOOLCHAIN_DIR/check-cppcheck.sh|$DRY_RUN_FLAG|"
)

run_task_graph "$RESULTS_DIR" "${TOOLCHAIN_TASKS[@]}"

# --- 7. Phase 2/3: project dependencies (independent package managers, parallel) ---
log_banner "2/3 - Project Dependencies"

DEP_DIR="$SCRIPT_DIR/scripts/dependencies"

VCPKG_DEPS_ARGS="--manifest-dir $VCPKG_MANIFEST_DIR --vcpkg-root $VCPKG_ROOT $DRY_RUN_FLAG"
[ -n "$VCPKG_INSTALLED_DIR" ] && VCPKG_DEPS_ARGS="$VCPKG_DEPS_ARGS --installed-dir $VCPKG_INSTALLED_DIR"

DEP_TASKS=(
    "CargoDeps|$DEP_DIR/parse-cargo.sh|--workspace-root $CARGO_WORKSPACE_ROOT $DRY_RUN_FLAG|"
    "vcpkgDeps|$DEP_DIR/parse-vcpkg.sh|$VCPKG_DEPS_ARGS|"
)

run_parallel_tasks "$RESULTS_DIR" "${DEP_TASKS[@]}"

# --- 8. Phase 3/3: CMake configuration ---
log_banner "3/3 - CMake Configuration"

CMAKE_CONFIGURE_ARGS=(
    --source-dir "$CMAKE_SOURCE_DIR"
    --build-dir "$CMAKE_BUILD_DIR"
    --vcpkg-root "$VCPKG_ROOT"
    --manifest-dir "$VCPKG_MANIFEST_DIR"
)
[ -n "$VCPKG_INSTALLED_DIR" ] && CMAKE_CONFIGURE_ARGS+=(--installed-dir "$VCPKG_INSTALLED_DIR")
[ "$DRY_RUN" = "true" ] && CMAKE_CONFIGURE_ARGS+=(--dry-run)
CMAKE_CONFIGURE_ARGS+=(--result-file "$RESULTS_DIR/CMakeConfigure.result")

"$SCRIPT_DIR/scripts/final/cmake-configure.sh" "${CMAKE_CONFIGURE_ARGS[@]}"

# --- 9. Summary table ---
show_summary_table "$RESULTS_DIR"

if [ "$CV_SUMMARY_FAILED" -gt 0 ]; then
    exit 1
fi
exit 0
