#Requires -Version 7.0
<#
.NOTES
    Depends on Rustup.
    Cargo is installed together with Rustup.
#>

param(
    [string]$RequiredVersion = '1.82.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command cargo -ErrorAction SilentlyContinue

    if ($cmd) {
        (& cargo --version) 2>&1
    }
    else {
        $null
    }
}

$upstreamInstall = {

    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        throw "Rustup is missing."
    }

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Cargo is missing even though Rustup is installed."
    }

    # Cargo is shipped with Rustup.
    # Rustup update is handled by check-rustup.ps1.
}

Invoke-ToolCheck `
    -ToolName 'Cargo' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId $null `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall
