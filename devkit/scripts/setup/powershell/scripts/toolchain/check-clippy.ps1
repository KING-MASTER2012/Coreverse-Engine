#Requires -Version 7.0
<#
.NOTES
    Depends on Rustup (task graph: DependsOn = 'Rustup').
    Clippy is a rustup *component*, not a standalone package — installed via
    `rustup component add clippy`, not winget/cargo install.
#>

param(
    [string]$RequiredVersion = '0.1.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cmd) {
        (& cargo clippy --version) 2>&1 | Select-Object -First 1
    } else {
        $null
    }
}

$upstreamInstall = {
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        throw 'Rustup is missing.'
    }

    & rustup component add clippy
    if ($LASTEXITCODE -ne 0) {
        throw "rustup component add clippy failed (exit code $LASTEXITCODE)."
    }
}

Invoke-ToolCheck `
    -ToolName 'Clippy' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId $null `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall
