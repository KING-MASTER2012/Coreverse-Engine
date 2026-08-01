#!/usr/bin/env bash
# Coreverse Bootstrap - Homebrew wrapper (macOS).
# IMPORTANT: Homebrew refuses to run as root by design - never prefix these with sudo.
# Sourced by every script that may need it; do not execute directly.

# ensure_homebrew_installed
# Installs Homebrew itself if it is missing, then makes it available on PATH
# for the current process (covers both Apple Silicon and Intel install paths).
ensure_homebrew_installed() {
    if command -v brew >/dev/null 2>&1; then
        return 0
    fi

    log_warning "Homebrew not found, installing it first..." "Homebrew"
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    if [ -x /opt/homebrew/bin/brew ]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [ -x /usr/local/bin/brew ]; then
        eval "$(/usr/local/bin/brew shellenv)"
    fi

    command -v brew >/dev/null 2>&1
}

brew_update_index() {
    with_pkg_lock "brew update" >/dev/null 2>&1
}

brew_install() {
    local packages="$*"
    with_pkg_lock "brew install $packages"
}

brew_upgrade() {
    local packages="$*"
    # 'brew upgrade' fails if the formula isn't installed yet; treat that as non-fatal
    # since the caller already tried brew_install as a first attempt.
    with_pkg_lock "brew upgrade $packages || true"
}
