#!/usr/bin/env bash
# CoreVerse Bootstrap - OS platform, Linux distro family, and architecture detection.
# Sets global variables: OS_PLATFORM, OS_DISTRO, OS_ARCH, PKG_MANAGER.
# Sourced by every script; do not execute directly.

# detect_os
# Sets OS_PLATFORM (linux|macos|unknown), OS_DISTRO, OS_ARCH (x86_64|arm64),
# and PKG_MANAGER (pacman|apt|dnf|zypper|brew|none).
# Returns 1 if the platform/distro is not supported.
detect_os() {
    local raw_arch
    raw_arch=$(uname -m)
    case "$raw_arch" in
        x86_64|amd64) OS_ARCH="x86_64" ;;
        arm64|aarch64) OS_ARCH="arm64" ;;
        *) OS_ARCH="$raw_arch" ;;
    esac

    local kernel
    kernel=$(uname -s)

    if [ "$kernel" = "Darwin" ]; then
        OS_PLATFORM="macos"
        OS_DISTRO="macos"
        PKG_MANAGER="brew"
        return 0
    fi

    if [ "$kernel" != "Linux" ]; then
        OS_PLATFORM="unknown"
        OS_DISTRO="unsupported"
        PKG_MANAGER="none"
        return 1
    fi

    OS_PLATFORM="linux"

    if [ ! -f /etc/os-release ]; then
        OS_DISTRO="unsupported"
        PKG_MANAGER="none"
        return 1
    fi

    # shellcheck disable=SC1091
    . /etc/os-release
    local id="${ID:-}"
    local id_like="${ID_LIKE:-}"
    local haystack=" $id $id_like "

    case "$haystack" in
        *" arch "*)
            OS_DISTRO="arch"; PKG_MANAGER="pacman" ;;
        *" debian "*)
            OS_DISTRO="debian"; PKG_MANAGER="apt" ;;
        *" fedora "*)
            OS_DISTRO="fedora"; PKG_MANAGER="dnf" ;;
        *" suse "*|*" opensuse "*)
            OS_DISTRO="opensuse"; PKG_MANAGER="zypper" ;;
        *" rhel "*)
            OS_DISTRO="rhel"; PKG_MANAGER="dnf" ;;
        *)
            case "$id" in
                arch|cachyos|endeavouros|garuda|manjaro)
                    OS_DISTRO="arch"; PKG_MANAGER="pacman" ;;
                debian|ubuntu|linuxmint|pop|kali)
                    OS_DISTRO="debian"; PKG_MANAGER="apt" ;;
                fedora)
                    OS_DISTRO="fedora"; PKG_MANAGER="dnf" ;;
                opensuse*|sles)
                    OS_DISTRO="opensuse"; PKG_MANAGER="zypper" ;;
                rhel|rocky|almalinux|centos)
                    OS_DISTRO="rhel"; PKG_MANAGER="dnf" ;;
                *)
                    OS_DISTRO="unsupported"; PKG_MANAGER="none" ;;
            esac
            ;;
    esac

    # Kali is Debian-based (ID_LIKE=debian) but gets its own display label.
    if [ "$id" = "kali" ]; then
        OS_DISTRO="kali"
        PKG_MANAGER="apt"
    fi

    [ "$OS_DISTRO" = "unsupported" ] && return 1
    return 0
}

# is_admin_capable
# Returns 0 if this script can escalate privileges (root already, or sudo works).
is_admin_capable() {
    if [ "$(id -u)" -eq 0 ]; then
        return 0
    fi
    command -v sudo >/dev/null 2>&1
}

# sudo_cmd
# Prints "sudo " (with trailing space) when privilege escalation is actually
# needed, or nothing when already running as root. This avoids hard-failing
# on minimal/root-only environments (e.g. containers) where `sudo` itself
# isn't installed but isn't needed anyway.
sudo_cmd() {
    if [ "$(id -u)" -eq 0 ]; then
        printf ''
    else
        printf 'sudo '
    fi
}
