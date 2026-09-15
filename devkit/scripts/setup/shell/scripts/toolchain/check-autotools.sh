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
# and every subsequent CMake Configure step then fails too, since
# vcpkg_installed/ never gets populated.
#
# No official upstream installer exists for these (they're OS-packaged
# tools), so - like GDB/cppcheck - this only goes through the package
# manager. autoconf-archive ships M4 macros only (no binary), so it can't be
# probed on PATH; it's included in the package-manager install list
# regardless, keyed off whether autoconf/automake/libtoolize are missing.

TOOL_NAME="Autotools"
REQUIRED_VERSION=$(read_config_min_version autotools)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="2.60"
PKG_NAME=$(read_config_pkg_name autotools)
[ -z "$PKG_NAME" ] && PKG_NAME="autoconf autoconf-archive automake libtool"
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --required-version) REQUIRED_VERSION="$2"; shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        *) shift ;;
    esac
done

get_version_raw() {
    # Only report "found" once all three binaries vcpkg's autoreconf step
    # needs are present; a partial install (e.g. autoconf but no libtool) is
    # exactly as broken as none, so it must still fall through to install.
    if command -v autoconf >/dev/null 2>&1 && \
       command -v automake >/dev/null 2>&1 && \
       command -v libtoolize >/dev/null 2>&1; then
        autoconf --version 2>/dev/null | head -n1
    fi
}

upstream_install() {
    log_error "No upstream installer for Autotools - it must come from the package manager ($PKG_MANAGER)." "$TOOL_NAME"
    return 1
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
