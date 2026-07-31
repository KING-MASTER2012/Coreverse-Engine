#Requires -Version 7.0
param(
    [string]$RequiredVersion = '1.22.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command go -ErrorAction SilentlyContinue
    if ($cmd) { (& go version) 2>&1 } else { $null }
}

$upstreamInstall = {
    $releases = Invoke-RestMethod -Uri 'https://go.dev/dl/?mode=json' -Headers @{ 'User-Agent' = 'CoreverseBootstrap' }
    $stable = $releases | Where-Object { $_.stable } | Select-Object -First 1
    if (-not $stable) { throw 'Recommended release not found on go.dev.' }

    $asset = $stable.files | Where-Object { $_.os -eq 'windows' -and $_.arch -eq 'amd64' -and $_.kind -eq 'installer' } | Select-Object -First 1
    if (-not $asset) { throw 'Suitable Go Windows MSI package not found.' }

    $installerPath = Join-Path $env:TEMP $asset.filename
    Invoke-WebRequest -Uri "https://go.dev/dl/$($asset.filename)" -OutFile $installerPath
    Start-Process -FilePath 'msiexec.exe' -ArgumentList "/i `"$installerPath`" /quiet" -Wait
    Sync-EnvironmentPath
}

Invoke-ToolCheck -ToolName 'Go' -RequiredVersion $RequiredVersion -DryRun:$DryRun `
    -WingetId 'GoLang.Go' -GetVersionRaw $getVersion -UpstreamInstall $upstreamInstall
