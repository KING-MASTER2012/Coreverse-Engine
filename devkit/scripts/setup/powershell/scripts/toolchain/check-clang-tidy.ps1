#Requires -Version 7.0
<#
.NOTES
    Depends on LLVM/Clang (task graph: DependsOn = 'LLVM/Clang').
    clang-tidy ships inside the same LLVM.LLVM winget package / apt.llvm.org
    bootstrap install as clang itself, so this script does not perform a
    separate top-level install — it only verifies the binary is present and
    repairs the shared LLVM install if it is missing (e.g. a partial/older
    install that predates clang-tools-extra being pulled in).
#>

param(
    [string]$RequiredVersion = '22.1.8',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command clang-tidy -ErrorAction SilentlyContinue
    if ($cmd) {
        (& clang-tidy --version) 2>&1 | Select-Object -First 2 | Select-Object -Last 1
    } else {
        $defaultBin = 'C:\Program Files\LLVM\bin\clang-tidy.exe'
        if (Test-Path $defaultBin) {
            (& $defaultBin --version) 2>&1 | Select-Object -First 2 | Select-Object -Last 1
        } else {
            $null
        }
    }
}

$upstreamInstall = {
    # clang-tidy has no separate upstream installer — it is part of the LLVM
    # release. If it's missing, the LLVM install itself needs repairing.
    throw 'clang-tidy not found. Re-run check-llvm.ps1 (or repair the LLVM install manually) — clang-tidy ships as part of the same LLVM release.'
}

Invoke-ToolCheck `
    -ToolName 'clang-tidy' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId 'LLVM.LLVM' `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall
