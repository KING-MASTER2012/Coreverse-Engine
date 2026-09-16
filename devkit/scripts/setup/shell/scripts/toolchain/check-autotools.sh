#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

# Several vcpkg ports (e.g. libb2, pulled in transitively by qtbase ->
# widgets) are autotools-based and shell out to the SYSTEM's autoconf/
# automake/libtool during their build step - vcpkg itself never installs
# these, it only assumes they're already on PATH (see
# vcpkg_make_configure -> vcpkg_run_autoreconf). Without this check,
# "2/3 - Project Dependencies" (parse-vcpkg.sh's `vcpkg install`) fails with:
#   "libb2 currently requires the following programs from the system
#    package manager: autoconf autoconf-archive automake libtoolize"
#
# IMPORTANT - why this does NOT use invoke_tool_check's normal
# found-on-PATH-skip-install flow (like GDB/cppcheck do): autoconf-archive
# ships only M4 macros (ax_*.m4 under .../share/aclocal), it has no binary
# to put on PATH. Many CI images (incl. GitHub's ubuntu-latest) ship
# autoconf/automake/libtool preinstalled but NOT autoconf-archive. A
# "command -v autoconf/automake/libtoolize" check therefore reports the
# toolchain complete while autoconf-archive - the thing libb2's autoreconf
# step actually needs the macros from - is still missing, and vcpkg
# install fails downstream anyway. So instead of gating on presence, this
# script always (idempotently) runs the package-manager install for the
# full set; re-installing already-present packages is a fast no-op.

TOOL_NAME="Autotools"
PKG_NAME=$(read_config_pkg_name autotools)
[ -z "$PKG_NAME" ] && PKG_NAME="autoconf autoconf-archive automake libtool"
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        # Accepted for CLI consistency with the other check-*.sh scripts;
        # unused here since there's no single version to gate on (see above).
        --required-version) shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        *) shift ;;
    esac
done

binaries_present() {
    command -v autoconf >/dev/null 2>&1 && \
    command -v automake >/dev/null 2>&1 && \
    command -v libtoolize >/dev/null 2>&1
}

if [ "$DRY_RUN" = "true" ]; then
    log_info "[DryRun] Would ensure installed via package manager: $PKG_NAME." "$TOOL_NAME"
    write_result "$RESULT_FILE" "$TOOL_NAME" "DryRun" ""
    exit 0
fi

if [ -z "${PKG_MANAGER:-}" ] || [ "$PKG_MANAGER" = "none" ]; then
    log_error "No supported package manager detected - cannot install $PKG_NAME." "$TOOL_NAME"
    write_result "$RESULT_FILE" "$TOOL_NAME" "Failed" ""
    exit 0
fi

log_info "Ensuring $PKG_NAME are installed (package manager: $PKG_MANAGER)..." "$TOOL_NAME"
pkg_update_index
# shellcheck disable=SC2086
pkg_install $PKG_NAME >/dev/null 2>&1

if binaries_present; then
    RAW=$(autoconf --version 2>/dev/null | head -n1)
    log_success "Ready: $RAW (autoconf-archive installed alongside, no binary to verify)." "$TOOL_NAME"
    write_result "$RESULT_FILE" "$TOOL_NAME" "Installed" "$RAW"
else
    log_error "autoconf/automake/libtoolize still not found on PATH after install." "$TOOL_NAME"
    write_result "$RESULT_FILE" "$TOOL_NAME" "Failed" ""
fi
