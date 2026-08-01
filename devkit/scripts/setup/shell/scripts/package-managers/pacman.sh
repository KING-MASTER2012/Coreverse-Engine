#!/usr/bin/env bash
# Coreverse Bootstrap - pacman wrapper (Arch and Arch-based distros).
# Sourced by every script that may need it; do not execute directly.

pacman_update_index() {
    with_pkg_lock "$(sudo_cmd)pacman -Sy --noconfirm" >/dev/null 2>&1
}

pacman_install() {
    local packages="$*"
    with_pkg_lock "$(sudo_cmd)pacman -S --noconfirm --needed $packages"
}

pacman_upgrade() {
    # pacman -S already resolves to the latest version in the sync database.
    pacman_install "$@"
}
