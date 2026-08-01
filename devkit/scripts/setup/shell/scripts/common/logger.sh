#!/usr/bin/env bash
# Coreverse Bootstrap - shared logging functions.
# Sourced by every script; do not execute directly.

CV_COLOR_RESET='\033[0m'
CV_COLOR_CYAN='\033[0;36m'
CV_COLOR_GREEN='\033[0;32m'
CV_COLOR_YELLOW='\033[0;33m'
CV_COLOR_RED='\033[0;31m'
CV_COLOR_WHITE='\033[1;37m'
CV_COLOR_GRAY='\033[0;90m'

# log_info <message> [source]
log_info() {
    local message="$1" source="${2:-}"
    local tag=""
    [ -n "$source" ] && tag="[$source] "
    printf "🔵 %b[INFO] %s%b\n" "$CV_COLOR_CYAN" "${tag}${message}" "$CV_COLOR_RESET"
}

log_success() {
    local message="$1" source="${2:-}"
    local tag=""
    [ -n "$source" ] && tag="[$source] "
    printf "🟢 %b[SUCCESS] %s%b\n" "$CV_COLOR_GREEN" "${tag}${message}" "$CV_COLOR_RESET"
}

log_warning() {
    local message="$1" source="${2:-}"
    local tag=""
    [ -n "$source" ] && tag="[$source] "
    printf "🟡 %b[WARNING] %s%b\n" "$CV_COLOR_YELLOW" "${tag}${message}" "$CV_COLOR_RESET"
}

log_error() {
    local message="$1" source="${2:-}"
    local tag=""
    [ -n "$source" ] && tag="[$source] "
    printf "🔴 %b[ERROR] %s%b\n" "$CV_COLOR_RED" "${tag}${message}" "$CV_COLOR_RESET" >&2
}

log_plain() {
    local message="$1" source="${2:-}"
    local tag=""
    [ -n "$source" ] && tag="[$source] "
    printf "%s%s\n" "$tag" "$message"
}

log_banner() {
    local title="$1"
    local line
    line=$(printf '=%.0s' $(seq 1 $((${#title} + 4))))
    printf "\n%b%s\n  %s\n%s%b\n" "$CV_COLOR_WHITE" "$line" "$title" "$line" "$CV_COLOR_RESET"
}
