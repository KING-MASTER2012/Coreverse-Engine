#!/usr/bin/env bash
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/../common/logger.sh"

# NOTE: this task depends on 'Git' in the task graph (DependsOn = Git).
# vcpkg is installed project-locally (not system-wide) and is identical on
# Linux/macOS: sync the submodule + run bootstrap-vcpkg.sh.
# Matches project-paths.json's "vcpkgRoot" key (renamed from "vcpkgInstallDir").
#
# vcpkg is a git SUBMODULE of this repository (.gitmodules, third_party/vcpkg)
# and the commit that the repository points to IS the pinned vcpkg version -
# the same rule check-vcpkg.ps1 and the CI use. It used to clone the latest
# vcpkg and `git pull` it, which made every machine (and the CI's binary cache
# key) depend on the day the script ran. To move to another vcpkg version:
#     git -C third_party/vcpkg fetch origin
#     git -C third_party/vcpkg checkout <commit>
#     git add third_party/vcpkg      # and commit it
# (`builtin-baseline` in vcpkg.json is separate: it pins the PORT versions.)

TOOL_NAME="vcpkg"
VCPKG_ROOT="third_party/vcpkg"
MANIFEST_FILE="vcpkg.json"
DRY_RUN="false"
RESULT_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --vcpkg-root) VCPKG_ROOT="$2"; shift 2 ;;
        --dry-run) DRY_RUN="true"; shift ;;
        --result-file) RESULT_FILE="$2"; shift 2 ;;
        --required-version) shift 2 ;;  # accepted for CLI consistency, unused
        *) shift ;;
    esac
done

write_result() {
    local file="$1" tool="$2" status="$3" version="$4"
    if [ -z "$file" ]; then
        return 0
    fi
    mkdir -p "$(dirname "$file")"
    printf 'TOOL=%s\nSTATUS=%s\nVERSION=%s\n' "$tool" "$status" "$version" > "$file"
}

if ! command -v git >/dev/null 2>&1; then
    log_error "Git not found. vcpkg needs Git to clone." "$TOOL_NAME"
    write_result "$RESULT_FILE" "$TOOL_NAME" "Failed" ""
    exit 0
fi

# The commit the repository pins (the index entry of the submodule; empty when
# this is not a git checkout of the repository, e.g. a source archive).
PINNED_SHA=""
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    PINNED_SHA="$(git ls-files -s -- "$VCPKG_ROOT" 2>/dev/null | awk '$1 == "160000" { print $2; exit }')"
fi

VCPKG_BIN="$VCPKG_ROOT/vcpkg"
ALREADY_CLONED="false"
[ -x "$VCPKG_BIN" ] && ALREADY_CLONED="true"

if [ "$ALREADY_CLONED" = "true" ]; then
    PREVIOUS_VERSION=$("$VCPKG_BIN" version 2>/dev/null | head -n1)
    log_info "Found: $VCPKG_ROOT ($PREVIOUS_VERSION)" "$TOOL_NAME"
else
    log_warning "$VCPKG_ROOT not found, will perform the initial setup." "$TOOL_NAME"
fi

if [ "$DRY_RUN" = "true" ]; then
    if [ -n "$PINNED_SHA" ]; then
        log_info "[DryRun] Would run: git submodule update --init (pinned commit ${PINNED_SHA}) + re-bootstrap." "$TOOL_NAME"
    else
        log_info "[DryRun] Would run: clone + checkout builtin-baseline + bootstrap (not a git checkout of the repository)." "$TOOL_NAME"
    fi
    write_result "$RESULT_FILE" "$TOOL_NAME" "DryRun" ""
    exit 0
fi

if [ -n "$PINNED_SHA" ]; then
    # Only when the submodule is checked out: in an empty (uninitialized)
    # directory `git -C` would report the HEAD of THIS repository instead.
    CURRENT_SHA=""
    [ -e "$VCPKG_ROOT/.git" ] && CURRENT_SHA="$(git -C "$VCPKG_ROOT" rev-parse HEAD 2>/dev/null || true)"
    if [ -n "$CURRENT_SHA" ] && [ "$CURRENT_SHA" != "$PINNED_SHA" ]; then
        log_warning "vcpkg is at ${CURRENT_SHA} but the repository pins ${PINNED_SHA}; moving it to the pinned commit. If you updated vcpkg on purpose, run 'git add $VCPKG_ROOT' and commit it first." "$TOOL_NAME"
    fi
    log_info "Syncing the vcpkg submodule with the commit pinned by the repository..." "$TOOL_NAME"
    # Not shallow: vcpkg's builtin-baseline resolves an arbitrary historical
    # commit (whatever vcpkg.json pins) via `git show <sha>:versions/baseline.json`
    # against THIS clone. A shallow clone only carries one commit, so any
    # baseline other than it is missing from the local object database even
    # though it is a perfectly valid commit upstream - this is exactly what
    # produced "path 'versions/baseline.json' exists on disk, but not in
    # '<commit>'". vcpkg's own docs call out that shallow clones of the
    # registry are unsupported for this reason.
    if ! SYNC_OUTPUT="$(git submodule update --init -- "$VCPKG_ROOT" 2>&1)"; then
        log_error "git submodule update failed for $VCPKG_ROOT: $SYNC_OUTPUT" "$TOOL_NAME"
        write_result "$RESULT_FILE" "$TOOL_NAME" "Failed" ""
        exit 0
    fi
else
    # Not a git checkout of the repository (source archive, copied tree): there
    # is no submodule to sync, so clone and pin to the builtin-baseline of
    # vcpkg.json - still a fixed commit, never "latest".
    BASELINE="$(sed -n 's/.*"builtin-baseline"[[:space:]]*:[[:space:]]*"\([0-9a-f]\{40\}\)".*/\1/p' "$MANIFEST_FILE" 2>/dev/null | sed -n 1p)"
    if [ -z "$BASELINE" ]; then
        log_error "Not a git checkout and no builtin-baseline in $MANIFEST_FILE: cannot tell which vcpkg commit to use." "$TOOL_NAME"
        write_result "$RESULT_FILE" "$TOOL_NAME" "Failed" ""
        exit 0
    fi
    if [ ! -d "$VCPKG_ROOT/.git" ]; then
        PARENT_DIR=$(dirname "$VCPKG_ROOT")
        [ -n "$PARENT_DIR" ] && [ "$PARENT_DIR" != "." ] && mkdir -p "$PARENT_DIR"
        log_info "Cloning https://github.com/microsoft/vcpkg.git ..." "$TOOL_NAME"
        # NOT --depth 1, see the comment above.
        git clone https://github.com/microsoft/vcpkg.git "$VCPKG_ROOT" >/dev/null 2>&1
    fi
    log_info "Checking out the builtin-baseline commit $BASELINE ..." "$TOOL_NAME"
    if ! git -C "$VCPKG_ROOT" cat-file -e "${BASELINE}^{commit}" 2>/dev/null; then
        git -C "$VCPKG_ROOT" fetch origin >/dev/null 2>&1
    fi
    if ! git -C "$VCPKG_ROOT" checkout --quiet "$BASELINE" 2>/dev/null; then
        log_error "Could not check out $BASELINE in $VCPKG_ROOT." "$TOOL_NAME"
        write_result "$RESULT_FILE" "$TOOL_NAME" "Failed" ""
        exit 0
    fi
fi

log_info "Running bootstrap-vcpkg.sh..." "$TOOL_NAME"
( cd "$VCPKG_ROOT" && ./bootstrap-vcpkg.sh -disableMetrics ) >/dev/null 2>&1

if [ -x "$VCPKG_BIN" ]; then
    FINAL_VERSION=$("$VCPKG_BIN" version 2>/dev/null | head -n1)
    if [ "$ALREADY_CLONED" = "true" ]; then
        log_success "Ready (updated): $FINAL_VERSION" "$TOOL_NAME"
        write_result "$RESULT_FILE" "$TOOL_NAME" "Upgraded" "$FINAL_VERSION"
    else
        log_success "Ready (installed): $FINAL_VERSION" "$TOOL_NAME"
        write_result "$RESULT_FILE" "$TOOL_NAME" "Installed" "$FINAL_VERSION"
    fi
else
    log_error "vcpkg binary not found after bootstrap." "$TOOL_NAME"
    write_result "$RESULT_FILE" "$TOOL_NAME" "Failed" ""
fi
