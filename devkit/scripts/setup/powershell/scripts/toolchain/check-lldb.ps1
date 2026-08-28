#Requires -Version 7.0
<#
.NOTES
    Depends on LLVM/Clang (task graph: DependsOn = 'LLVM/Clang').
    lldb ships inside the same LLVM.LLVM winget package / apt.llvm.org
    bootstrap install as clang itself — see check-clang-tidy.ps1 for the same
    rationale. On Windows the primary debugger remains the Visual Studio
    Debugger (see check-vs.ps1); lldb is provided here for LLVM-based
    diagnostics/verification workflows.
#>

param(
    [string]$RequiredVersion = '22.1.8',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"

$getVersion = {
    $cmd = Get-Command lldb -ErrorAction SilentlyContinue
    if ($cmd) {
        (& lldb --version) 2>&1 | Select-Object -First 1
    } else {
        $defaultBin = 'C:\Program Files\LLVM\bin\lldb.exe'
        if (Test-Path $defaultBin) {
            (& $defaultBin --version) 2>&1 | Select-Object -First 1
        } else {
            $null
        }
    }
}

$upstreamInstall = {
    throw 'lldb not found. Re-run check-llvm.ps1 (or repair the LLVM install manually) — lldb ships as part of the same LLVM release.'
}

Invoke-ToolCheck `
    -ToolName 'LLDB' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId 'LLVM.LLVM' `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall
