#Requires -Version 7.0

param(
    [string]$RequiredVersion = '1.27.0',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command rustup -ErrorAction SilentlyContinue
    if ($cmd) {
        (& rustup --version) 2>&1 | Select-Object -First 1
    } else {
        $null
    }
}

$upstreamInstall = {

    $installerPath = Join-Path $env:TEMP 'rustup-init.exe'

    Invoke-WebRequest `
        -Uri 'https://win.rustup.rs/x86_64' `
        -OutFile $installerPath

    & $installerPath `
        -y `
        --default-toolchain stable `
        --profile default

    if ($LASTEXITCODE -ne 0) {
        throw "rustup-init failed (exit code $LASTEXITCODE)."
    }

    $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'

    if (Test-Path $cargoBin) {

        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')

        if ($userPath -notlike "*$cargoBin*") {
            [Environment]::SetEnvironmentVariable(
                'Path',
                "$userPath;$cargoBin",
                'User'
            )
        }

        $env:Path += ";$cargoBin"
    }

    & rustup self update
    if ($LASTEXITCODE -ne 0) {
        throw "rustup self update failed (exit code $LASTEXITCODE)."
    }

    & rustup default stable
    if ($LASTEXITCODE -ne 0) {
        throw "rustup default stable failed (exit code $LASTEXITCODE)."
    }

    & rustup update stable
    if ($LASTEXITCODE -ne 0) {
        throw "rustup update stable failed (exit code $LASTEXITCODE)."
    }
}

Invoke-ToolCheck `
    -ToolName 'Rustup' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId 'Rustlang.Rustup' `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall
