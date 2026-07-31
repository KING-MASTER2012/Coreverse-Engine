#Requires -Version 7.0
param(
    [string]$RequiredVersion = '17.0.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command clang -ErrorAction SilentlyContinue
    if ($cmd) { (& clang --version) 2>&1 | Select-Object -First 1 } else { $null }
}

$upstreamInstall = {
    $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/llvm/llvm-project/releases/latest' `
        -Headers @{ 'User-Agent' = 'CoreverseBootstrap' }
    $asset = $release.assets | Where-Object { $_.name -match 'win64\.exe$' } | Select-Object -First 1
    if (-not $asset) { throw 'Suitable LLVM Windows installer not found.' }

    $installerPath = Join-Path $env:TEMP $asset.name
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $installerPath
    # LLVM's NSIS installer supports silent installation with the /S command.
    Start-Process -FilePath $installerPath -ArgumentList '/S' -Wait
    Sync-EnvironmentPath

    # The LLVM installer may not always add it to the PATH; let's add the default location as well.
    $defaultBin = 'C:\Program Files\LLVM\bin'
    if ((Test-Path $defaultBin) -and ($env:Path -notlike "*$defaultBin*")) {
        $env:Path = "$env:Path;$defaultBin"
    }
}

Invoke-ToolCheck -ToolName 'LLVM/Clang' -RequiredVersion $RequiredVersion -DryRun:$DryRun `
    -WingetId 'LLVM.LLVM' -GetVersionRaw $getVersion -UpstreamInstall $upstreamInstall
