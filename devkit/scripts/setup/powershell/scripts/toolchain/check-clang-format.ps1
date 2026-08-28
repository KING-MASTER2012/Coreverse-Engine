#Requires -Version 7.0
<#
.NOTES
    Depends on LLVM/Clang (task graph: DependsOn = 'LLVM/Clang').
    clang-format ships inside the same LLVM.LLVM winget package / apt.llvm.org
    bootstrap install as clang itself — see check-clang-tidy.ps1 for the same
    rationale. MSVC has no native formatter, so clang-format is the formatter
    for both the Windows and Linux/macOS toolchains.
#>

param(
    [string]$RequiredVersion = '22.1.8',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command clang-format -ErrorAction SilentlyContinue
    if ($cmd) {
        (& clang-format --version) 2>&1 | Select-Object -First 1
    } else {
        $defaultBin = 'C:\Program Files\LLVM\bin\clang-format.exe'
        if (Test-Path $defaultBin) {
            (& $defaultBin --version) 2>&1 | Select-Object -First 1
        } else {
            $null
        }
    }
}

$upstreamInstall = {
    throw 'clang-format not found. Re-run check-llvm.ps1 (or repair the LLVM install manually) — clang-format ships as part of the same LLVM release.'
}

Invoke-ToolCheck `
    -ToolName 'clang-format' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId 'LLVM.LLVM' `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall
