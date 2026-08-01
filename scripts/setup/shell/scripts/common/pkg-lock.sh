#!/usr/bin/env bash
# Coreverse Bootstrap - package-manager mutex.
#
# Multiple toolchain checks run in parallel, but apt/dnf/pacman/zypper (and to a
# lesser extent brew) cannot handle concurrent install invocations - they lock
# their own database and the second call would fail or hang. This wraps any
# package-manager command so only one runs at a time, while everything else
# (version detection, upstream downloads) still runs fully in parallel.
#
# Deliberately uses `mkdir` (atomic on every POSIX filesystem) instead of
# `flock`, because macOS does not ship `flock` by default.
# Sourced by every script; do not execute directly.

CV_PKG_LOCK_DIR="${TMPDIR:-/tmp}/.coreverse-bootstrap-pkg.lock.d"

# with_pkg_lock <command string>
# Runs the given command string (via eval) while holding the lock.
with_pkg_lock() {
    local cmd="$*"
    local waited=0
    local max_wait=300

    while ! mkdir "$CV_PKG_LOCK_DIR" 2>/dev/null; do
        sleep 1
        waited=$((waited + 1))
        if [ "$waited" -ge "$max_wait" ]; then
            log_error "Timed out waiting for the package-manager lock." "PkgLock"
            return 1
        fi
    done

    eval "$cmd"
    local rc=$?

    rmdir "$CV_PKG_LOCK_DIR" 2>/dev/null

    return $rc
}
