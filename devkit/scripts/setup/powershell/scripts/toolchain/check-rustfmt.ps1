#Requires -Version 7.0
<#
.NOTES
    Depends on Rustup (task graph: DependsOn = 'Rustup').
    rustfmt is a rustup *component*, not a standalone package — installed via
    `rustup component add rustfmt`, not winget/cargo install.
#>

param(
    [string]$RequiredVersion = '1.7.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cmd) {
        (& cargo fmt --version) 2>&1 | Select-Object -First 1
    } else {
        $null
    }
}

$upstreamInstall = {
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        throw 'Rustup is missing.'
    }

    & rustup component add rustfmt
    if ($LASTEXITCODE -ne 0) {
        throw "rustup component add rustfmt failed (exit code $LASTEXITCODE)."
    }
}

Invoke-ToolCheck `
    -ToolName 'rustfmt' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId $null `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall
