#!/usr/bin/env bash
# Coreverse Bootstrap - dnf wrapper (Fedora, RHEL, Rocky Linux, AlmaLinux).
# Falls back to yum on older RHEL/CentOS releases where dnf is unavailable.
# Sourced by every script that may need it; do not execute directly.

_dnf_binary() {
    if command -v dnf >/dev/null 2>&1; then
        printf 'dnf'
    else
        printf 'yum'
    fi
}

dnf_update_index() {
    local bin
    bin=$(_dnf_binary)
    with_pkg_lock "$(sudo_cmd)$bin makecache -y" >/dev/null 2>&1
}

dnf_install() {
    local bin packages
    bin=$(_dnf_binary)
    packages="$*"
    with_pkg_lock "$(sudo_cmd)$bin install -y $packages"
}

dnf_upgrade() {
    local bin packages
    bin=$(_dnf_binary)
    packages="$*"
    with_pkg_lock "$(sudo_cmd)$bin upgrade -y $packages"
}
