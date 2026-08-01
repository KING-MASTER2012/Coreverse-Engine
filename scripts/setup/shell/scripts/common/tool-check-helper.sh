#!/usr/bin/env bash
# Coreverse Bootstrap - shared check/install flow used by every toolchain/check-*.sh
# script. Implements the hybrid version strategy:
#   1) Already on PATH and version sufficient?  -> leave it alone.
#   2) Not sufficient: try the distro/Homebrew package manager.
#   3) Still not sufficient: fall back to the tool's official upstream installer.
#      If upstream leaves the tool below the required version, continue with a
#      WARNING (per project policy) instead of stopping the whole bootstrap.
#
# Every check-*.sh script defines get_version_raw() and upstream_install() as
# shell functions, then calls invoke_tool_check with their names.

SCRIPT_DIR_TOOL_CHECK="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/logger.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/os-detect.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/version-compare.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/pkg-lock.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/pkg-dispatch.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/../package-managers/pacman.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/../package-managers/apt.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/../package-managers/dnf.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/../package-managers/zypper.sh"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_TOOL_CHECK/../package-managers/brew.sh"

detect_os

# Every check-*.sh lives at setup/shell/scripts/toolchain/; the shared config
# lives at setup/config/. Resolve it relative to this file's own location so it
# works regardless of the caller's current working directory.
CV_CONFIG_DIR="$SCRIPT_DIR_TOOL_CHECK/../../../config"
CV_TOOL_VERSIONS_JSON="$CV_CONFIG_DIR/tool-versions.json"

# read_config_min_version <tool_key>
# Prints the tool's minVersion from tool-versions.json, or nothing if jq/the
# config file/the key is unavailable (caller should fall back to a safety default).
read_config_min_version() {
    local tool_key="$1"
    command -v jq >/dev/null 2>&1 || return 0
    jq -r --arg t "$tool_key" '.[$t].minVersion // empty' "$CV_TOOL_VERSIONS_JSON" 2>/dev/null
}

# read_config_pkg_name <tool_key>
# Prints the tool's package name for the currently detected $PKG_MANAGER, or
# nothing if unavailable.
read_config_pkg_name() {
    local tool_key="$1"
    command -v jq >/dev/null 2>&1 || return 0
    jq -r --arg t "$tool_key" --arg m "$PKG_MANAGER" '.[$t].pkg[$m] // empty' "$CV_TOOL_VERSIONS_JSON" 2>/dev/null
}

# write_result <file> <tool> <status> <version>
write_result() {
    local file="$1" tool="$2" status="$3" version="$4"
    if [ -z "$file" ]; then
        return 0
    fi
    mkdir -p "$(dirname "$file")"
    printf 'TOOL=%s\nSTATUS=%s\nVERSION=%s\n' "$tool" "$status" "$version" > "$file"
}

# invoke_tool_check <tool_name> <required_version> <pkg_name> <get_version_func> <upstream_install_func> <dry_run> <result_file>
invoke_tool_check() {
    local tool_name="$1"
    local required_version="$2"
    local pkg_name="$3"
    local get_version_func="$4"
    local upstream_install_func="$5"
    local dry_run="$6"
    local result_file="$7"

    local raw
    # Multi-package names (e.g. "clang llvm") are passed comma-joined through
    # task argument strings to survive naive space-based word-splitting; decode
    # them back to space-separated form here, right before actual use.
    pkg_name="${pkg_name//,/ }"

    raw=$("$get_version_func" 2>/dev/null)

    if [ -n "$raw" ]; then
        log_info "Found: $raw" "$tool_name"
    else
        log_warning "Not found on PATH." "$tool_name"
    fi

    if [ -n "$raw" ] && version_ge "$raw" "$required_version"; then
        log_success "Version is sufficient (>= $required_version)." "$tool_name"
        write_result "$result_file" "$tool_name" "OK" "$raw"
        return 0
    fi

    if [ "$dry_run" = "true" ]; then
        log_info "[DryRun] Would install/upgrade (package manager -> upstream)." "$tool_name"
        write_result "$result_file" "$tool_name" "DryRun" ""
        return 0
    fi

    # --- Step 1: distro / Homebrew package manager ---
    if [ -n "$pkg_name" ] && [ "$PKG_MANAGER" != "none" ]; then
        log_info "Trying the package manager ($PKG_MANAGER)..." "$tool_name"
        pkg_update_index
        if [ -n "$raw" ]; then
            # shellcheck disable=SC2086
            pkg_upgrade $pkg_name >/dev/null 2>&1
        else
            # shellcheck disable=SC2086
            pkg_install $pkg_name >/dev/null 2>&1
        fi

        raw=$("$get_version_func" 2>/dev/null)
        if [ -n "$raw" ] && version_ge "$raw" "$required_version"; then
            log_success "Installed/upgraded via package manager: $raw" "$tool_name"
            write_result "$result_file" "$tool_name" "Installed" "$raw"
            return 0
        fi
    fi

    # --- Step 2: upstream fallback ---
    log_warning "Package manager was not enough, falling back to upstream." "$tool_name"

    if "$upstream_install_func"; then

        raw=$("$get_version_func" 2>/dev/null)

        if [ -n "$raw" ] && version_ge "$raw" "$required_version"; then
            log_success "Installed/upgraded via upstream: $raw" "$tool_name"
            write_result "$result_file" "$tool_name" "Installed" "$raw"

        elif [ -n "$raw" ]; then
            log_warning "Installed but below the required version: $raw (required >= $required_version). Continuing." "$tool_name"
            write_result "$result_file" "$tool_name" "Warning" "$raw"

        else
            log_error "Tool is still not found on PATH after installation." "$tool_name"
            write_result "$result_file" "$tool_name" "Failed" ""
        fi

    else
        exit_code=$?

        log_error "Upstream installation failed (exit code: $exit_code)." "$tool_name"
        write_result "$result_file" "$tool_name" "Failed" ""
    fi

    return 0
}
