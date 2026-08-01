#!/usr/bin/env bash
# CoreVerse Bootstrap - zypper wrapper (openSUSE).
# Sourced by every script that may need it; do not execute directly.

zypper_update_index() {
    with_pkg_lock "$(sudo_cmd)zypper --non-interactive refresh" >/dev/null 2>&1
}

zypper_install() {
    local packages="$*"
    with_pkg_lock "$(sudo_cmd)zypper --non-interactive install $packages"
}

zypper_upgrade() {
    local packages="$*"
    with_pkg_lock "$(sudo_cmd)zypper --non-interactive update $packages"
}
