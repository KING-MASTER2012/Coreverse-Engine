#Requires -Version 7.0
<#
.NOTES
    Depends on Cargo (task graph: DependsOn = 'Cargo').
    cbindgen is installed via `cargo install --locked cbindgen`. Must be
    >= 0.28 — earlier versions cannot parse the `#[unsafe(no_mangle)]`
    attribute syntax the 2024 edition requires (see cmake/FfiHeader.cmake
    and engine/rust/crates/ffi/src/lib.rs for the full rationale). Most OS
    package managers still ship older, which is exactly why this is a
    cargo-installed tool rather than a pacman/apt/dnf/zypper/brew package.
    Uses the 'cargo-install' tool lock (see tool-lock.ps1) because mdBook
    also DependsOn = 'Cargo' and would otherwise run `cargo install`
    concurrently with this one, which cargo doesn't handle cleanly.
#>

param(
    [string]$RequiredVersion = '0.28.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command cbindgen -ErrorAction SilentlyContinue
    if ($cmd) {
        (& cbindgen --version) 2>&1 | Select-Object -First 1
    } else {
        $null
    }
}

$upstreamInstall = {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Cargo is missing.'
    }

    & cargo install --locked cbindgen
    if ($LASTEXITCODE -ne 0) {
        throw "cargo install --locked cbindgen failed (exit code $LASTEXITCODE)."
    }
}

Invoke-ToolCheck `
    -ToolName 'cbindgen' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId $null `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall `
    -ToolLockName 'cargo-install'
