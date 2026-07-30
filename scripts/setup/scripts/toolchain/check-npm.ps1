#Requires -Version 7.0
<#
.NOTES
    This task depends on the 'Node.js' task in the task graph (DependsOn = @('Node.js')).
    npm comes with Node.js; there is no separate winget/upstream installation.
#>
param(
    [string]$RequiredVersion = '10.0.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command npm -ErrorAction SilentlyContinue
    if ($cmd) { (& npm --version) 2>&1 } else { $null }
}

$upstreamInstall = {
    $nodeCmd = Get-Command node -ErrorAction SilentlyContinue
    if (-not $nodeCmd) {
        throw 'Node.js not found. npm comes with Node.js, Node.js must be installed first.'
    }
    & npm install -g npm@latest | Out-Null
    Sync-EnvironmentPath
}

Invoke-ToolCheck -ToolName 'npm' -RequiredVersion $RequiredVersion -DryRun:$DryRun `
    -WingetId $null -GetVersionRaw $getVersion -UpstreamInstall $upstreamInstall
