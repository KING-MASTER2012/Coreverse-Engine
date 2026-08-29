#Requires -Version 7.0
<#
.NOTES
    Depends on LLVM/Clang (task graph: DependsOn = 'LLVM/Clang').
    lldb ships inside the same LLVM.LLVM winget package / apt.llvm.org
    bootstrap install as clang itself — see check-clang-tidy.ps1 for the same
    rationale. On Windows the primary debugger remains the Visual Studio
    Debugger (see check-vs.ps1); lldb is provided here for LLVM-based
    diagnostics/verification workflows.

    In addition to the lldb binary itself, LLDB's Python scripting bridge
    (used by pretty-printers and any scripted breakpoints/commands) needs a
    compatible Python 3 interpreter, plus whatever pip packages
    tool-versions.json lists under lldb.pythonPackages. Nothing previously
    checked this side of it at all — `lldb --version` succeeding says
    nothing about whether `import lldb` will work in Python. This is a
    best-effort, warn-and-continue check (see python-check.ps1), consistent
    with this project's policy of not hard-failing the whole bootstrap over
    a secondary/optional capability.
#>

param(
    [string]$RequiredVersion = '22.1.8',
    [switch]$DryRun,
    # From tool-versions.json's lldb.pythonPackages - kept config-driven so
    # a project can add/remove required pip packages without touching this
    # script, same as RequiredVersion above.
    [string[]]$RequiredPythonPackages = @()
)

. "$PSScriptRoot/../common/tool-check-helper.ps1"
. "$PSScriptRoot/../common/python-check.ps1"

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

$result = Invoke-ToolCheck `
    -ToolName 'LLDB' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId 'LLVM.LLVM' `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall

# --- Python scripting bridge check (does not affect $result.Status above -
#     the lldb binary itself is what that status reflects; this is an
#     additional, separately-logged, best-effort check). ---
if (-not $DryRun -and $result.Status -in @('OK', 'Installed', 'Upgraded', 'Warning')) {

    $python = Find-PythonExecutable
    if (-not $python) {
        Write-WarningLog -Message "No Python 3 interpreter found - LLDB's Python scripting support (pretty-printers, scripted breakpoints) will not work. Install Python 3 to enable it." -Source 'LLDB'
    } else {
        # LLVM's Windows installer places its bundled lldb.py under one of
        # these, depending on release layout - try both, plus whatever's
        # already importable via the normal PYTHONPATH.
        $extraPythonPath = @(
            'C:\Program Files\LLVM\lib\site-packages'
            'C:\Program Files\LLVM\bin\Lib\site-packages'
        )

        if (Test-PythonModuleImportable -Python $python -ModuleName 'lldb' -ExtraPythonPath $extraPythonPath) {
            Write-SuccessLog -Message "LLDB Python scripting support verified ($($python.Version))." -Source 'LLDB'
        } else {
            Write-WarningLog -Message "Found $($python.Version) but 'import lldb' failed - LLDB's Python scripting support may not work. Continuing." -Source 'LLDB'
        }

        foreach ($pkg in $RequiredPythonPackages) {
            if (-not (Test-PythonModuleImportable -Python $python -ModuleName $pkg -ExtraPythonPath $extraPythonPath)) {
                Write-WarningLog -Message "Required python package '$pkg' not importable." -Source 'LLDB'
                Install-PythonPipPackage -Python $python -PackageName $pkg -Source 'LLDB' | Out-Null
            }
        }
    }
} elseif ($DryRun -and $RequiredPythonPackages.Count -gt 0) {
    Write-InfoLog -Message "[DryRun] Would verify Python 3 + import checks for: $($RequiredPythonPackages -join ', ')" -Source 'LLDB'
}

return $result
