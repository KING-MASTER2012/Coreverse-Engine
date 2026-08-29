#Requires -Version 7.0
<#
.SYNOPSIS
    Coreverse Bootstrap - reusable Python interpreter / module-import checks.

.DESCRIPTION
    Until now, every check-*.ps1 only ever verified a tool's own binary and
    version (Invoke-ToolCheck / Test-VersionAtLeast). Nothing verified the
    *side* dependencies some tools need to actually work once installed -
    e.g. LLDB's Python scripting bridge needs a compatible Python 3
    interpreter (and sometimes extra pip packages) that lldb.exe's own
    --version check says nothing about.

    This file is deliberately generic - not LLDB-specific - so any future
    check-*.ps1 that needs "is python3 present, and can it import module X"
    can dot-source this and call the functions below instead of writing its
    own ad-hoc version of the same check.

    Sourced by check-*.ps1 scripts that need it; not intended to be run
    directly.
#>

# Find-PythonExecutable
# Looks for a Python 3 interpreter, preferring the `py` launcher (the most
# reliable way to pin Python 3 specifically on Windows, since `python` alone
# can resolve to the Microsoft Store alias or a Python 2 install on older
# systems), then falling back to `python3`/`python` on PATH.
# Returns a PSCustomObject { File, Args, Version } or $null if none found.
function Find-PythonExecutable {
    $candidates = @(
        @{ File = 'py';      Args = @('-3') }
        @{ File = 'python3'; Args = @() }
        @{ File = 'python';  Args = @() }
    )

    foreach ($c in $candidates) {
        $cmd = Get-Command $c.File -ErrorAction SilentlyContinue
        if (-not $cmd) { continue }

        $verArgs = @($c.Args) + '--version'
        try {
            $verOutput = & $c.File @verArgs 2>&1
        } catch {
            continue
        }

        $verLine = ($verOutput | Select-Object -First 1).ToString().Trim()
        if ($verLine -notmatch 'Python 3') { continue }

        return [PSCustomObject]@{
            File    = $c.File
            Args    = $c.Args
            Version = $verLine
        }
    }

    return $null
}

# Invoke-PythonCommand <Python> <PythonArgs> [-ExtraPythonPath <dirs>]
# Runs the given interpreter with the given args, optionally prepending
# extra directories to PYTHONPATH for the duration of the call (used to
# point at a tool-bundled site-packages dir, e.g. LLVM's lldb.py, without
# permanently polluting the environment). Returns $true on exit code 0.
function Invoke-PythonCommand {
    param(
        [Parameter(Mandatory)][PSCustomObject]$Python,
        [Parameter(Mandatory)][string[]]$PythonArgs,
        [string[]]$ExtraPythonPath
    )

    $prevPythonPath = $env:PYTHONPATH
    try {
        if ($ExtraPythonPath -and $ExtraPythonPath.Count -gt 0) {
            $existing = @($ExtraPythonPath | Where-Object { Test-Path $_ -ErrorAction SilentlyContinue })
            if ($existing.Count -gt 0) {
                $parts = $existing + @($prevPythonPath) | Where-Object { $_ }
                $env:PYTHONPATH = $parts -join ';'
            }
        }

        $allArgs = @($Python.Args) + $PythonArgs
        & $Python.File @allArgs *> $null
        return $LASTEXITCODE -eq 0
    } finally {
        $env:PYTHONPATH = $prevPythonPath
    }
}

# Test-PythonModuleImportable <Python> <ModuleName> [-ExtraPythonPath <dirs>]
# Returns $true if `import <ModuleName>` succeeds under the given interpreter.
function Test-PythonModuleImportable {
    param(
        [Parameter(Mandatory)][PSCustomObject]$Python,
        [Parameter(Mandatory)][string]$ModuleName,
        [string[]]$ExtraPythonPath
    )

    return Invoke-PythonCommand -Python $Python -PythonArgs @('-c', "import $ModuleName") -ExtraPythonPath $ExtraPythonPath
}

# Install-PythonPipPackage <Python> <PackageName> [-Source <log tag>]
# Best-effort `pip install --user`; logs and returns $false rather than
# throwing, matching this project's "warn and continue" policy for
# secondary/optional dependencies.
function Install-PythonPipPackage {
    param(
        [Parameter(Mandatory)][PSCustomObject]$Python,
        [Parameter(Mandatory)][string]$PackageName,
        [string]$Source
    )

    Write-InfoLog -Message "Installing python package '$PackageName' (pip install --user)..." -Source $Source
    $ok = Invoke-PythonCommand -Python $Python -PythonArgs @('-m', 'pip', 'install', '--user', '--quiet', $PackageName)

    if ($ok) {
        Write-SuccessLog -Message "Installed python package '$PackageName'." -Source $Source
    } else {
        Write-WarningLog -Message "Could not install python package '$PackageName' (pip install failed). Continuing." -Source $Source
    }

    return $ok
}
