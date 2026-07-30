#Requires -Version 7.0
param(
    [string]$RequiredVersion = '2.40.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command git -ErrorAction SilentlyContinue
    if ($cmd) { (& git --version) 2>&1 } else { $null }
}

$upstreamInstall = {
    $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/git-for-windows/git/releases/latest' `
        -Headers @{ 'User-Agent' = 'CoreverseBootstrap' }
    $asset = $release.assets | Where-Object { $_.name -match '64-bit\.exe$' } | Select-Object -First 1
    if (-not $asset) { throw 'Suitable Git for Windows installer not found.' }

    $installerPath = Join-Path $env:TEMP $asset.name
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $installerPath
    Start-Process -FilePath $installerPath -ArgumentList '/VERYSILENT', '/NORESTART' -Wait
    Sync-EnvironmentPath
}

Invoke-ToolCheck -ToolName 'Git' -RequiredVersion $RequiredVersion -DryRun:$DryRun `
    -WingetId 'Git.Git' -GetVersionRaw $getVersion -UpstreamInstall $upstreamInstall
