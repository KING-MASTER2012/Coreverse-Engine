#Requires -Version 7.0
param(
    [string]$RequiredVersion = '1.11.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command ninja -ErrorAction SilentlyContinue
    if ($cmd) { (& ninja --version) 2>&1 | Select-Object -First 1 } else { $null }
}

$upstreamInstall = {
    $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/ninja-build/ninja/releases/latest' `
        -Headers @{ 'User-Agent' = 'CoreverseBootstrap' }
    $asset = $release.assets | Where-Object { $_.name -eq 'ninja-win.zip' } | Select-Object -First 1
    if (-not $asset) { throw 'Suitable ninja-win.zip package not found.' }

    $zipPath = Join-Path $env:TEMP 'ninja-win.zip'
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zipPath

    $installDir = Join-Path $env:LOCALAPPDATA 'CoreverseBootstrap\tools\ninja'
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Expand-Archive -Path $zipPath -DestinationPath $installDir -Force

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($userPath -notlike "*$installDir*") {
        [Environment]::SetEnvironmentVariable('Path', "$userPath;$installDir", 'User')
    }
    $env:Path = "$env:Path;$installDir"
}

Invoke-ToolCheck -ToolName 'Ninja' -RequiredVersion $RequiredVersion -DryRun:$DryRun `
    -WingetId 'Ninja-build.Ninja' -GetVersionRaw $getVersion -UpstreamInstall $upstreamInstall
