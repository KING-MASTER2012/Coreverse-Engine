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

function Find-LldbBinary {
    # Shared by $getVersion and the post-check Warning-downgrade below, so
    # both agree on where lldb would live.
    $cmd = Get-Command lldb -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }
    $defaultBin = 'C:\Program Files\LLVM\bin\lldb.exe'
    if (Test-Path $defaultBin) {
        return $defaultBin
    }
    return $null
}

$getVersion = {
    $lldbPath = Find-LldbBinary
    if (-not $lldbPath) {
        return $null
    }

    # lldb.exe can be present on disk but still fail to launch (its Python
    # scripting bridge needs a specific python3*.dll at runtime - see the
    # Warning-downgrade below) - check $LASTEXITCODE explicitly instead of
    # trusting `2>&1`'s captured text as if it were a version string. That
    # used to make an "unable to find python311.dll" error look like a
    # version to Invoke-ToolCheck, and made the eventual Failed/Warning
    # outcome depend on wording rather than on what actually happened.
    $output = & $lldbPath --version 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $output) {
        return $null
    }
    $output | Select-Object -First 1
}

$upstreamInstall = {
    # Nothing left to actually install here: lldb ships inside the same
    # LLVM.LLVM winget package as clang (see this file's top comment), and
    # Invoke-ToolCheck's own winget step above already re-ran that upgrade.
    # If lldb still isn't usable at this point, it's a runtime-dependency
    # problem (handled as a Warning below), not a "never installed" one -
    # so this deliberately does nothing rather than throwing unconditionally
    # regardless of which of those two situations it actually is.
}

$result = Invoke-ToolCheck `
    -ToolName 'LLDB' `
    -RequiredVersion $RequiredVersion `
    -DryRun:$DryRun `
    -WingetId 'LLVM.LLVM' `
    -GetVersionRaw $getVersion `
    -UpstreamInstall $upstreamInstall

# LLDB is explicitly documented above as a secondary/optional capability -
# this project's policy is warn-and-continue for it, not hard-fail. But
# Invoke-ToolCheck (shared by every check-*.ps1) can't tell "the binary is
# genuinely missing" apart from "the binary exists but $getVersion
# correctly returned $null because it failed to run" - both look like "no
# version, nothing installable" to it, and it reports 'Failed' either way.
# Downgrade here, using the same Find-LldbBinary check $getVersion used,
# rather than changing that shared helper's behavior for every other tool.
if (-not $DryRun -and $result.Status -eq 'Failed' -and (Find-LldbBinary)) {
    $result.Status = 'Warning'
    Write-WarningLog -Message "lldb.exe is present but failed to run (likely a missing runtime dependency, e.g. python311.dll) - continuing; this doesn't block the rest of Bootstrap." -Source 'LLDB'
}

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
