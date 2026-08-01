#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/tool-check-helper.sh"

TOOL_NAME="Git"
REQUIRED_VERSION=$(read_config_min_version git)
[ -z "$REQUIRED_VERSION" ] && REQUIRED_VERSION="2.55.0"
PKG_NAME=$(read_config_pkg_name git)
[ -z "$PKG_NAME" ] && PKG_NAME="git"
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --required-version) REQUIRED_VERSION="$2"; shift 2 ;;
        --pkg-name) PKG_NAME="$2"; shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        *) shift ;;
    esac
done

get_version_raw() {
    command -v git >/dev/null 2>&1 && git --version 2>/dev/null
}

upstream_install() {
    case "$OS_PLATFORM" in
        macos)
            # Xcode Command Line Tools are the real upstream source of git on macOS.
            log_info "Requesting Xcode Command Line Tools (includes git)..." "$TOOL_NAME"
            xcode-select --install >/dev/null 2>&1 || true
            log_warning "If a Command Line Tools dialog appeared, finish it and re-run bootstrap." "$TOOL_NAME"
            ;;
        linux)
            if [ "$OS_DISTRO" = "debian" ] || [ "$OS_DISTRO" = "kali" ]; then
                # git-core PPA ships newer git than the default Ubuntu/Debian repos.
                if command -v add-apt-repository >/dev/null 2>&1; then
                    with_pkg_lock "sudo add-apt-repository -y ppa:git-core/ppa" >/dev/null 2>&1
                    apt_update_index
                    apt_install git
                else
                    log_error "add-apt-repository not available, cannot add the git-core PPA." "$TOOL_NAME"
                    return 1
                fi
            else
                log_error "No further upstream source for git on this distro; the package manager already provides the official build." "$TOOL_NAME"
                return 1
            fi
            ;;
        *)
            return 1 ;;
    esac
}

invoke_tool_check "$TOOL_NAME" "$REQUIRED_VERSION" "$PKG_NAME" get_version_raw upstream_install "$DRY_RUN" "$RESULT_FILE"
