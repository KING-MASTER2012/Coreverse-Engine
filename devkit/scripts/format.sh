#!/usr/bin/env bash
# Coreverse - formats (or checks) every Rust and C++ source in the project.
#
#   Rust : cargo fmt --all                       (whole Cargo workspace)
#   C++  : clang-format -i  on engine/cpp and tests/cpp
#          (*.cpp *.cc *.cxx *.h *.hpp *.hxx - exactly the set the CI
#          "Formatting" workflow checks, see .github/workflows/formatting.yml)
#
# Usage:
#   devkit/scripts/format.sh                 format everything in place
#   devkit/scripts/format.sh --check         change nothing, exit 1 if anything is
#                                            not formatted (this is what CI runs)
#   devkit/scripts/format.sh --rust-only     only Rust
#   devkit/scripts/format.sh --cpp-only      only C++
#   CLANG_FORMAT=/path/to/clang-format devkit/scripts/format.sh
#                                            use a specific clang-format binary
#
# clang-format's output differs between LLVM major versions, so the version
# matters: the project pins it in devkit/scripts/setup/config/tool-versions.json
# (clangFormat.minVersion) and CI installs exactly that one. This script looks
# for a binary with the same major version first and warns if it can only
# find a different one.
#
# Exit codes: 0 = ok, 1 = formatting problems (--check) or a tool failed,
#             2 = bad usage / a required tool is missing.
#
# Written to also run on macOS's stock bash 3.2 (no associative arrays, no
# mapfile, no empty-array expansions).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

LOGGER="$SCRIPT_DIR/setup/shell/scripts/common/logger.sh"
if [ -f "$LOGGER" ]; then
    # shellcheck disable=SC1090
    . "$LOGGER"
else
    log_info() { printf '[INFO] %s\n' "$1"; }
    log_success() { printf '[SUCCESS] %s\n' "$1"; }
    log_warning() { printf '[WARNING] %s\n' "$1"; }
    log_error() { printf '[ERROR] %s\n' "$1" >&2; }
    log_plain() { printf '%s\n' "$1"; }
    log_banner() { printf '\n== %s ==\n' "$1"; }
fi

usage() {
    sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

CHECK="false"
DO_RUST="true"
DO_CPP="true"

for arg in "$@"; do
    case "$arg" in
        --check | -c) CHECK="true" ;;
        --rust-only) DO_CPP="false" ;;
        --cpp-only) DO_RUST="false" ;;
        --help | -h)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown argument: $arg"
            usage >&2
            exit 2
            ;;
    esac
done

if [ "$DO_RUST" = "false" ] && [ "$DO_CPP" = "false" ]; then
    log_error "--rust-only and --cpp-only cancel each other out; nothing to do."
    exit 2
fi

cd "$REPO_ROOT" || exit 2

if [ "$CHECK" = "true" ]; then
    log_banner "Coreverse - format check (no files will be changed)"
else
    log_banner "Coreverse - format"
fi

RESULT=0

# ---------------------------------------------------------------------------
# Rust
# ---------------------------------------------------------------------------
format_rust() {
    log_info "Rust: cargo fmt --all"

    if ! command -v cargo > /dev/null 2>&1; then
        log_error "cargo was not found on PATH (install Rust via https://rustup.rs)."
        return 2
    fi
    if ! cargo fmt --version > /dev/null 2>&1; then
        log_error "rustfmt is not installed for the active toolchain. Run: rustup component add rustfmt"
        return 2
    fi

    if [ "$CHECK" = "true" ]; then
        if cargo fmt --all -- --check; then
            log_success "Rust: all files are formatted."
            return 0
        fi
        log_error "Rust: unformatted files found (see the diff above). Run devkit/scripts/format.sh to fix."
        return 1
    fi

    if cargo fmt --all; then
        log_success "Rust: formatted."
        return 0
    fi
    log_error "Rust: cargo fmt failed."
    return 1
}

# ---------------------------------------------------------------------------
# C++
# ---------------------------------------------------------------------------

# Prints "<major>.<minor>.<patch>" for a clang-format binary, or nothing.
clang_format_version() {
    "$1" --version 2> /dev/null | head -n1 | sed -n 's/.*version \([0-9][0-9]*\(\.[0-9][0-9]*\)*\).*/\1/p'
}

# Wanted clang-format version: tool-versions.json when jq is around, otherwise
# the value below (keep it in sync with tool-versions.json).
expected_clang_format_version() {
    local v=""
    local json="$SCRIPT_DIR/setup/config/tool-versions.json"
    if command -v jq > /dev/null 2>&1 && [ -f "$json" ]; then
        v="$(jq -r '.clangFormat.minVersion // empty' "$json" 2> /dev/null)"
    fi
    [ -z "$v" ] && v="22.1.8"
    printf '%s' "$v"
}

# Sets CF_BIN / CF_VERSION. Prefers a binary whose major version matches the
# expected one; falls back to the first clang-format found (with a warning).
find_clang_format() {
    local expected_major="$1"
    local candidates="" c v fallback="" fallback_version=""

    [ -n "${CLANG_FORMAT:-}" ] && candidates="$CLANG_FORMAT"$'\n'
    candidates="${candidates}clang-format-${expected_major}"$'\n'
    candidates="${candidates}clang-format"$'\n'
    candidates="${candidates}${HOME}/.local/coreverse-bootstrap/llvm/bin/clang-format"$'\n'
    # Homebrew's llvm formula is keg-only: it is never on PATH by default.
    candidates="${candidates}/opt/homebrew/opt/llvm/bin/clang-format"$'\n'
    candidates="${candidates}/usr/local/opt/llvm/bin/clang-format"$'\n'

    CF_BIN=""
    CF_VERSION=""

    while IFS= read -r c; do
        [ -z "$c" ] && continue
        if [ -x "$c" ]; then
            :
        elif command -v "$c" > /dev/null 2>&1; then
            c="$(command -v "$c")"
        else
            continue
        fi
        v="$(clang_format_version "$c")"
        [ -z "$v" ] && continue
        if [ "${v%%.*}" = "$expected_major" ]; then
            CF_BIN="$c"
            CF_VERSION="$v"
            return 0
        fi
        if [ -z "$fallback" ]; then
            fallback="$c"
            fallback_version="$v"
        fi
    done <<< "$candidates"

    if [ -n "$fallback" ]; then
        CF_BIN="$fallback"
        CF_VERSION="$fallback_version"
        return 0
    fi
    return 1
}

format_cpp() {
    local expected expected_major
    expected="$(expected_clang_format_version)"
    expected_major="${expected%%.*}"

    if ! find_clang_format "$expected_major"; then
        log_error "clang-format was not found. Install LLVM $expected (bootstrap does this), or set CLANG_FORMAT=/path/to/clang-format."
        return 2
    fi

    log_info "C++: using $CF_BIN (version $CF_VERSION)"
    if [ "${CF_VERSION%%.*}" != "$expected_major" ]; then
        log_warning "The project uses clang-format $expected (CI checks with it), but this is $CF_VERSION."
        log_warning "Its output can differ from CI's and CI may still report violations. Install LLVM $expected_major.x or set CLANG_FORMAT."
    fi

    local dirs="" d
    for d in engine/cpp tests/cpp; do
        [ -d "$d" ] && dirs="$dirs $d"
    done
    if [ -z "$dirs" ]; then
        log_warning "C++: neither engine/cpp nor tests/cpp exists, nothing to do."
        return 0
    fi

    local total=0 bad=0 bad_list="" file
    # shellcheck disable=SC2086
    while IFS= read -r -d '' file; do
        total=$((total + 1))
        if ! "$CF_BIN" --dry-run --Werror "$file" > /dev/null 2>&1; then
            bad=$((bad + 1))
            bad_list="${bad_list}  ${file}"$'\n'
            if [ "$CHECK" = "false" ]; then
                if ! "$CF_BIN" -i "$file"; then
                    log_error "C++: clang-format failed on $file"
                    return 1
                fi
            fi
        fi
    done < <(
        find $dirs -type f \
            \( -name '*.cpp' -o -name '*.cc' -o -name '*.cxx' \
            -o -name '*.h' -o -name '*.hpp' -o -name '*.hxx' \) \
            -print0
    )

    if [ "$CHECK" = "true" ]; then
        if [ "$bad" -eq 0 ]; then
            log_success "C++: all $total files are formatted."
            return 0
        fi
        log_error "C++: $bad of $total files are not formatted:"
        printf '%s' "$bad_list" >&2
        log_error "Run devkit/scripts/format.sh to fix them."
        return 1
    fi

    if [ "$bad" -eq 0 ]; then
        log_success "C++: all $total files were already formatted."
    else
        log_success "C++: reformatted $bad of $total files:"
        printf '%s' "$bad_list"
    fi
    return 0
}

# ---------------------------------------------------------------------------

if [ "$DO_RUST" = "true" ]; then
    format_rust
    rc=$?
    [ "$rc" -gt "$RESULT" ] && RESULT=$rc
fi

if [ "$DO_CPP" = "true" ]; then
    format_cpp
    rc=$?
    [ "$rc" -gt "$RESULT" ] && RESULT=$rc
fi

echo ""
if [ "$RESULT" -eq 0 ]; then
    if [ "$CHECK" = "true" ]; then
        log_success "Format check passed."
    else
        log_success "Done."
    fi
else
    log_error "Finished with problems (exit code $RESULT)."
fi

exit "$RESULT"
