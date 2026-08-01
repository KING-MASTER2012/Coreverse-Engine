#!/usr/bin/env bash
# Coreverse Bootstrap - dispatches generic pkg_* calls to the concrete package
# manager selected by detect_os() (in $PKG_MANAGER). This lets check-*.sh
# scripts call one generic function without caring which distro/OS they're on.
# Sourced by every script; do not execute directly.

pkg_install() {
    case "$PKG_MANAGER" in
        pacman) pacman_install "$@" ;;
        apt)    apt_install "$@" ;;
        dnf)    dnf_install "$@" ;;
        zypper) zypper_install "$@" ;;
        brew)   ensure_homebrew_installed && brew_install "$@" ;;
        *) log_error "No package manager available on this system." ; return 1 ;;
    esac
}

pkg_upgrade() {
    case "$PKG_MANAGER" in
        pacman) pacman_upgrade "$@" ;;
        apt)    apt_upgrade "$@" ;;
        dnf)    dnf_upgrade "$@" ;;
        zypper) zypper_upgrade "$@" ;;
        brew)   ensure_homebrew_installed && brew_upgrade "$@" ;;
        *) log_error "No package manager available on this system." ; return 1 ;;
    esac
}

pkg_update_index() {
    case "$PKG_MANAGER" in
        pacman) pacman_update_index ;;
        apt)    apt_update_index ;;
        dnf)    dnf_update_index ;;
        zypper) zypper_update_index ;;
        brew)   ensure_homebrew_installed && brew_update_index ;;
        *) return 1 ;;
    esac
}
