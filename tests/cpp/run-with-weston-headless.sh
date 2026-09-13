#!/usr/bin/env bash
# Runs a single test binary underneath a throwaway headless Weston
# compositor, so Wayland-backed tests (wayland_render_loop_test) can
# run unattended — no real display, DRM device, or X server needed.
# Invoked by tests/cpp/CMakeLists.txt's add_test(); not meant to be run
# by itself.
#
# Usage: run-with-weston-headless.sh <test-binary> [args passed through to it...]

set -uo pipefail

if [ $# -lt 1 ]; then
    echo "usage: run-with-weston-headless.sh <test-binary> [args...]" >&2
    exit 1
fi

if ! command -v weston >/dev/null 2>&1; then
    echo "run-with-weston-headless.sh: 'weston' not found on PATH — install it" \
         "(e.g. apt-get install weston) before running Wayland tests." >&2
    exit 1
fi

TEST_BIN="$1"
shift

# A dedicated, per-run socket name (PID-suffixed) so this can't collide
# with a compositor a developer already has running on the same
# machine, and a private XDG_RUNTIME_DIR so it doesn't depend on one
# already existing (CI containers frequently don't set one up).
SOCKET_NAME="coreverse-test-wayland-$$"
CREATED_RUNTIME_DIR=""
if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    CREATED_RUNTIME_DIR="$(mktemp -d)"
    export XDG_RUNTIME_DIR="$CREATED_RUNTIME_DIR"
fi
chmod 0700 "$XDG_RUNTIME_DIR"

WESTON_LOG="$(mktemp)"

weston --backend=headless-backend.so --socket="$SOCKET_NAME" --idle-time=0 >"$WESTON_LOG" 2>&1 &
WESTON_PID=$!

cleanup() {
    kill "$WESTON_PID" >/dev/null 2>&1
    wait "$WESTON_PID" 2>/dev/null
    rm -f "$WESTON_LOG"
    [ -n "$CREATED_RUNTIME_DIR" ] && rm -rf "$CREATED_RUNTIME_DIR"
}
trap cleanup EXIT

# Poll for the socket instead of a fixed sleep — CI runners vary a lot
# in how long compositor startup takes. ~10s ceiling, checked every 100ms.
SOCKET_PATH="$XDG_RUNTIME_DIR/$SOCKET_NAME"
for _ in $(seq 1 100); do
    if [ -S "$SOCKET_PATH" ]; then
        break
    fi
    if ! kill -0 "$WESTON_PID" 2>/dev/null; then
        echo "weston exited before creating its socket; weston log follows:" >&2
        cat "$WESTON_LOG" >&2
        exit 1
    fi
    sleep 0.1
done

if [ ! -S "$SOCKET_PATH" ]; then
    echo "timed out waiting for weston's Wayland socket at $SOCKET_PATH; weston log follows:" >&2
    cat "$WESTON_LOG" >&2
    exit 1
fi

export WAYLAND_DISPLAY="$SOCKET_NAME"
"$TEST_BIN" "$@"
STATUS=$?

if [ "$STATUS" -ne 0 ]; then
    echo "--- weston log (test exited $STATUS) ---" >&2
    cat "$WESTON_LOG" >&2
fi

exit "$STATUS"
