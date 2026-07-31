#Requires -Version 7.0
<#
.NOTES
    This task depends on the 'Rustup' task in the task graph (DependsOn = @('Rustup')).
    # Cargo comes with rustup; there is no separate winget/upstream setup.
#>
param(
    [string]$RequiredVersion = '1.82.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cmd) { (& cargo --version) 2>&1 } else { $null }
}

$upstreamInstall = {
    $rustupCmd = Get-Command rustup -ErrorAction SilentlyContinue
    if (-not $rustupCmd) {
        throw 'Rustup could not be found. Cargo arrives via Rustup; Rustup must have been established first.'
    }
    & rustup default stable | Out-Null
    & rustup update stable | Out-Null
}

Invoke-ToolCheck -ToolName 'Cargo' -RequiredVersion $RequiredVersion -DryRun:$DryRun `
    -WingetId $null -GetVersionRaw $getVersion -UpstreamInstall $upstreamInstall
