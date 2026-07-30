#Requires -Version 7.0
param(
    [string]$RequiredVersion = '3.28.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command cmake -ErrorAction SilentlyContinue
    if ($cmd) { (& cmake --version) 2>&1 | Select-Object -First 1 } else { $null }
}

$upstreamInstall = {
    $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/Kitware/CMake/releases/latest' `
        -Headers @{ 'User-Agent' = 'CoreVerseBootstrap' }
    $asset = $release.assets | Where-Object { $_.name -match 'windows-x86_64\.msi$' } | Select-Object -First 1
    if (-not $asset) { throw 'Suitable CMake Windows MSI package not found.' }

    $installerPath = Join-Path $env:TEMP $asset.name
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $installerPath
    Start-Process -FilePath 'msiexec.exe' -ArgumentList "/i `"$installerPath`" /quiet ADD_CMAKE_TO_PATH=System" -Wait
    Sync-EnvironmentPath
}

Invoke-ToolCheck -ToolName 'CMake' -RequiredVersion $RequiredVersion -DryRun:$DryRun `
    -WingetId 'Kitware.CMake' -GetVersionRaw $getVersion -UpstreamInstall $upstreamInstall
