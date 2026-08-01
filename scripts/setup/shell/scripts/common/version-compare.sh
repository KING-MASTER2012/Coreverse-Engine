#!/usr/bin/env bash
# Coreverse Bootstrap - version string extraction and comparison.
# Deliberately avoids `sort -V` (not available in macOS's default BSD `sort`)
# so this works identically on Linux and macOS.
# Sourced by every script; do not execute directly.

# extract_version <raw_string>
# Prints the first X.Y(.Z)(.W) pattern found in the input, or nothing.
extract_version() {
    printf '%s' "$1" | grep -oE '[0-9]+\.[0-9]+(\.[0-9]+)?(\.[0-9]+)?' | head -n1
}

# version_ge <current_raw> <required_raw>
# Returns 0 (true) if current >= required, 1 otherwise (including when either
# side fails to parse - fail safe, treated as "not sufficient").
version_ge() {
    local current required
    current=$(extract_version "$1")
    required=$(extract_version "$2")

    [ -z "$current" ] && return 1
    [ -z "$required" ] && return 1

    local IFS=.
    # shellcheck disable=SC2206
    local -a v1=($current)
    # shellcheck disable=SC2206
    local -a v2=($required)

    local i a b
    for i in 0 1 2 3; do
        a="${v1[$i]:-0}"
        b="${v2[$i]:-0}"
        a=$(printf '%s' "$a" | grep -oE '^[0-9]+')
        b=$(printf '%s' "$b" | grep -oE '^[0-9]+')
        a="${a:-0}"
        b="${b:-0}"

        if [ "$a" -gt "$b" ]; then
            return 0
        elif [ "$a" -lt "$b" ]; then
            return 1
        fi
    done

    return 0  # equal
}
