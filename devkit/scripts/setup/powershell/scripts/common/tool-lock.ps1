#Requires -Version 7.0
<#
.SYNOPSIS
    Coreverse Bootstrap - mutex for CLI tools that cannot handle concurrent
    invocations of themselves (cargo install, rustup component add, ...).

.DESCRIPTION
    Invoke-TaskGraph already keeps a global $completed map to figure out
    which *layer* a task belongs to, but two tasks in the same layer that
    both DependsOn the same thing (e.g. mdBook and cbindgen, both
    DependsOn = 'Cargo') still run concurrently via separate Start-ThreadJob
    runspaces. That's fine for most tools, but `cargo install` and
    `rustup component add` take a lock on their own shared state (the cargo
    registry/package cache, the rustup toolchain directory) - running two of
    them at once doesn't fail cleanly, it just makes one sit there waiting
    on the other, which is what shows up as "cargo locks itself".

    pkg-lock.sh (bash side) already solves this exact problem for the OS
    package managers (apt/pacman/dnf/zypper/brew can't handle concurrent
    installs either) with a single named mutex. This is the PowerShell/
    Windows equivalent, generalized so any check-*.ps1 can opt in for any
    tool by picking a lock name - not hardcoded to cargo/rustup specifically,
    so a future tool with the same problem just needs one line added.

    A named System.Threading.Mutex is used (not a script-scoped variable)
    because Invoke-TaskGraph/Invoke-ParallelTasks run each task in its own
    Start-ThreadJob runspace - plain PowerShell variables aren't shared
    across those, but a named Mutex is visible to every runspace in the
    session, which is exactly the scope needed here.

    Sourced by tool-check-helper.ps1; not intended to be run directly.
#>

# Enter-ToolLock <LockName> [-TimeoutSeconds <n>] [-Source <log tag>]
# Blocks until the named lock is free (or the timeout elapses). Returns the
# acquired Mutex object on success, or $null on timeout - callers must check
# for $null and must call Exit-ToolLock on whatever they get back.
function Enter-ToolLock {
    param(
        [Parameter(Mandatory)][string]$LockName,
        [int]$TimeoutSeconds = 300,
        [string]$Source
    )

    $mutexName = "CoreverseBootstrap-$LockName"
    $mutex = [System.Threading.Mutex]::new($false, $mutexName)

    $acquired = $false
    try {
        $acquired = $mutex.WaitOne([TimeSpan]::FromSeconds($TimeoutSeconds))
    } catch [System.Threading.AbandonedMutexException] {
        # Whoever held the lock exited without releasing it (e.g. crashed
        # mid cargo-install); we now own it and can proceed safely.
        $acquired = $true
    }

    if (-not $acquired) {
        Write-ErrorLog -Message "Timed out waiting for the '$LockName' lock (waited ${TimeoutSeconds}s). Another task is likely stuck holding it." -Source $Source
        $mutex.Dispose()
        return $null
    }

    return $mutex
}

# Exit-ToolLock <Mutex>
# Releases and disposes a Mutex obtained from Enter-ToolLock. Safe to call
# even if the mutex is already released; always call from a finally block.
function Exit-ToolLock {
    param(
        [Parameter(Mandatory)][System.Threading.Mutex]$Mutex
    )

    try {
        $Mutex.ReleaseMutex()
    } catch {
        # Already released / not owned by this thread - nothing to do.
    }
    $Mutex.Dispose()
}
