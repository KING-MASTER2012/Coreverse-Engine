#Requires -Version 7.0
param(
    [string]$RequiredVersion = '20.0.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command node -ErrorAction SilentlyContinue
    if ($cmd) { (& node --version) 2>&1 } else { $null }
}

$upstreamInstall = {
    $index = Invoke-RestMethod -Uri 'https://nodejs.org/dist/index.json' -Headers @{ 'User-Agent' = 'CoreverseBootstrap' }
    $lts = $index | Where-Object { $_.lts -ne $false } | Select-Object -First 1
    if (-not $lts) { throw 'LTS release not found on nodejs.org' }

    $version = $lts.version  # ör. "v20.15.1"
    $fileName = "node-$version-x64.msi"
    $installerPath = Join-Path $env:TEMP $fileName
    Invoke-WebRequest -Uri "https://nodejs.org/dist/$version/$fileName" -OutFile $installerPath
    Start-Process -FilePath 'msiexec.exe' -ArgumentList "/i `"$installerPath`" /quiet" -Wait
    Sync-EnvironmentPath
}

Invoke-ToolCheck -ToolName 'Node.js' -RequiredVersion $RequiredVersion -DryRun:$DryRun `
    -WingetId 'OpenJS.NodeJS.LTS' -GetVersionRaw $getVersion -UpstreamInstall $upstreamInstall
