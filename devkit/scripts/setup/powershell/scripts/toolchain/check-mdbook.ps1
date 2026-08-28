#Requires -Version 7.0
<#
.NOTES
    Depends on Cargo (task graph: DependsOn = 'Cargo').
    mdBook is installed via `cargo install --locked mdbook` — no winget
    package is used, to keep the version pinned the same way cbindgen is.
#>

param(
    [string]$RequiredVersion = '0.4.40',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command mdbook -ErrorAction SilentlyContinue
    if ($cmd) {
        (& mdbook --version) 2>&1 | Select-Object -First 1
    } else {
        $null
    }
}

$upstreamInstall = {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Cargo is missing.'
    }

    & cargo install --locked mdbook
    if ($LASTEXITCODE -ne 0) {
        throw "cargo install --locked mdbook failed (exit code $LASTEXITCODE)."
    }
}

Invoke-ToolCheck `
    -ToolName 'mdBook' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId $null `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall
