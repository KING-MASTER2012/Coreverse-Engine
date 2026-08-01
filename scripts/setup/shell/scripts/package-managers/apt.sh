#!/usr/bin/env bash
# CoreVerse Bootstrap - apt wrapper (Debian and Debian-based distros, incl. Kali).
# Sourced by every script that may need it; do not execute directly.

apt_update_index() {
    with_pkg_lock "$(sudo_cmd)apt-get update -y" >/dev/null 2>&1
}

apt_install() {
    local packages="$*"
    with_pkg_lock "$(sudo_cmd)env DEBIAN_FRONTEND=noninteractive apt-get install -y $packages"
}

apt_upgrade() {
    local packages="$*"
    with_pkg_lock "$(sudo_cmd)env DEBIAN_FRONTEND=noninteractive apt-get install -y --only-upgrade $packages"
}
