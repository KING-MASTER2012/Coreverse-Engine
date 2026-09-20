#Requires -Version 7.0
<#
.SYNOPSIS
    Coreverse - formats (or checks) every Rust and C++ source in the project.

.DESCRIPTION
    Rust : cargo fmt --all                       (whole Cargo workspace)
    C++  : clang-format -i  on engine/cpp and tests/cpp
           (*.cpp *.cc *.cxx *.h *.hpp *.hxx - exactly the set the CI
           "Formatting" workflow checks, see .github/workflows/formatting.yml)

    clang-format's output differs between LLVM major versions, so the version
    matters: the project pins it in
    devkit/scripts/setup/config/tool-versions.json (clangFormat.minVersion) and
    CI installs exactly that one. This script looks for a binary with the same
    major version first and warns if it can only find a different one.

    Exit codes: 0 = ok, 1 = formatting problems (-Check) or a tool failed,
                2 = a required tool is missing.

.PARAMETER Check
    Change nothing; exit 1 if anything is not formatted (this is what CI runs).

.PARAMETER RustOnly
    Only format/check Rust.

.PARAMETER CppOnly
    Only format/check C++.

.PARAMETER ClangFormat
    Path to a specific clang-format binary (the CLANG_FORMAT environment
    variable works too).

.EXAMPLE
    devkit/scripts/format.ps1

.EXAMPLE
    devkit/scripts/format.ps1 -Check
#>
[CmdletBinding()]
param(
    [switch]$Check,
    [switch]$RustOnly,
    [switch]$CppOnly,
    [string]$ClangFormat
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

$loggerPath = Join-Path $PSScriptRoot 'setup/powershell/scripts/common/logger.ps1'
if (Test-Path -LiteralPath $loggerPath) {
    . $loggerPath
}
else {
    function Write-InfoLog    { param([string]$Message) Write-Host "[INFO] $Message" }
    function Write-SuccessLog { param([string]$Message) Write-Host "[OK] $Message" -ForegroundColor Green }
    function Write-WarningLog { param([string]$Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
    function Write-ErrorLog   { param([string]$Message) Write-Host "[ERR] $Message" -ForegroundColor Red }
    function Write-PlainLog   { param([string]$Message) Write-Host $Message }
    function Write-Banner     { param([string]$Title) Write-Host ""; Write-Host "== $Title ==" }
}

if ($RustOnly -and $CppOnly) {
    Write-ErrorLog '-RustOnly and -CppOnly cancel each other out; nothing to do.'
    exit 2
}

$doRust = -not $CppOnly
$doCpp = -not $RustOnly

# ---------------------------------------------------------------------------
# Rust
# ---------------------------------------------------------------------------
function Invoke-RustFormat {
    Write-InfoLog 'Rust: cargo fmt --all'

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-ErrorLog 'cargo was not found on PATH (install Rust via https://rustup.rs).'
        return 2
    }

    & cargo fmt --version *> $null
    if ($LASTEXITCODE -ne 0) {
        Write-ErrorLog 'rustfmt is not installed for the active toolchain. Run: rustup component add rustfmt'
        return 2
    }

    if ($Check) {
        & cargo fmt --all -- --check
        if ($LASTEXITCODE -eq 0) {
            Write-SuccessLog 'Rust: all files are formatted.'
            return 0
        }
        Write-ErrorLog 'Rust: unformatted files found (see the diff above). Run devkit/scripts/format.ps1 to fix.'
        return 1
    }

    & cargo fmt --all
    if ($LASTEXITCODE -eq 0) {
        Write-SuccessLog 'Rust: formatted.'
        return 0
    }
    Write-ErrorLog 'Rust: cargo fmt failed.'
    return 1
}

# ---------------------------------------------------------------------------
# C++
# ---------------------------------------------------------------------------

# Returns "<major>.<minor>.<patch>" for a clang-format binary, or $null.
function Get-ClangFormatVersion {
    param([string]$Path)
    try {
        $line = (& $Path --version 2>$null | Select-Object -First 1)
    }
    catch {
        return $null
    }
    if ($line -match 'version (\d+(\.\d+)*)') {
        return $Matches[1]
    }
    return $null
}

# Wanted clang-format version: tool-versions.json, falling back to the value
# below (keep it in sync with tool-versions.json).
function Get-ExpectedClangFormatVersion {
    $json = Join-Path $PSScriptRoot 'setup/config/tool-versions.json'
    if (Test-Path -LiteralPath $json) {
        try {
            $v = (Get-Content -LiteralPath $json -Raw | ConvertFrom-Json).clangFormat.minVersion
            if ($v) { return [string]$v }
        }
        catch {
            # fall through to the default below
        }
    }
    return '22.1.8'
}

# Candidate clang-format binaries, best guess first.
function Get-ClangFormatCandidates {
    param([string]$ExpectedMajor)

    $candidates = [System.Collections.Generic.List[string]]::new()

    if ($ClangFormat) { $candidates.Add($ClangFormat) }
    if ($env:CLANG_FORMAT) { $candidates.Add($env:CLANG_FORMAT) }

    foreach ($name in @("clang-format-$ExpectedMajor", 'clang-format')) {
        $cmd = Get-Command $name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($cmd) { $candidates.Add($cmd.Source) }
    }

    # winget's LLVM package (what the Windows bootstrap installs).
    if ($env:ProgramFiles) {
        $candidates.Add((Join-Path $env:ProgramFiles 'LLVM/bin/clang-format.exe'))
    }

    # The clang-format that ships with Visual Studio's "C++ Clang tools".
    $vswhere = $null
    if (${env:ProgramFiles(x86)}) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    }
    if ($vswhere -and (Test-Path -LiteralPath $vswhere)) {
        $vsPath = (& $vswhere -latest -products '*' -property installationPath 2>$null | Select-Object -First 1)
        if ($vsPath) {
            $candidates.Add((Join-Path $vsPath 'VC/Tools/Llvm/x64/bin/clang-format.exe'))
        }
    }

    return $candidates
}

# Returns @{ Path; Version } - prefers a binary with the expected major
# version, otherwise the first one found - or $null if there is none.
function Find-ClangFormat {
    param([string]$ExpectedMajor)

    $fallback = $null
    foreach ($candidate in (Get-ClangFormatCandidates -ExpectedMajor $ExpectedMajor)) {
        if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { continue }
        $version = Get-ClangFormatVersion -Path $candidate
        if (-not $version) { continue }
        if (($version -split '\.')[0] -eq $ExpectedMajor) {
            return @{ Path = $candidate; Version = $version }
        }
        if (-not $fallback) {
            $fallback = @{ Path = $candidate; Version = $version }
        }
    }
    return $fallback
}

function Invoke-CppFormat {
    $expected = Get-ExpectedClangFormatVersion
    $expectedMajor = ($expected -split '\.')[0]

    $cf = Find-ClangFormat -ExpectedMajor $expectedMajor
    if (-not $cf) {
        Write-ErrorLog "clang-format was not found. Install LLVM $expected (bootstrap does this), or pass -ClangFormat <path> / set CLANG_FORMAT."
        return 2
    }

    Write-InfoLog "C++: using $($cf.Path) (version $($cf.Version))"
    if (($cf.Version -split '\.')[0] -ne $expectedMajor) {
        Write-WarningLog "The project uses clang-format $expected (CI checks with it), but this is $($cf.Version)."
        Write-WarningLog "Its output can differ from CI's and CI may still report violations. Install LLVM $expectedMajor.x or pass -ClangFormat."
    }

    $extensions = @('.cpp', '.cc', '.cxx', '.h', '.hpp', '.hxx')
    $files = @(
        foreach ($dir in @('engine/cpp', 'tests/cpp')) {
            $path = Join-Path $RepoRoot $dir
            if (Test-Path -LiteralPath $path -PathType Container) {
                Get-ChildItem -LiteralPath $path -Recurse -File |
                    Where-Object { $extensions -contains $_.Extension.ToLowerInvariant() }
            }
        }
    ) | Sort-Object FullName

    if ($files.Count -eq 0) {
        Write-WarningLog 'C++: neither engine/cpp nor tests/cpp contains C++ files, nothing to do.'
        return 0
    }

    $bad = [System.Collections.Generic.List[string]]::new()
    foreach ($file in $files) {
        $rel = [System.IO.Path]::GetRelativePath($RepoRoot, $file.FullName) -replace '\\', '/'

        & $cf.Path --dry-run --Werror $file.FullName *> $null
        if ($LASTEXITCODE -ne 0) {
            $bad.Add($rel)
            if (-not $Check) {
                & $cf.Path -i $file.FullName
                if ($LASTEXITCODE -ne 0) {
                    Write-ErrorLog "C++: clang-format failed on $rel"
                    return 1
                }
            }
        }
    }

    if ($Check) {
        if ($bad.Count -eq 0) {
            Write-SuccessLog "C++: all $($files.Count) files are formatted."
            return 0
        }
        Write-ErrorLog "C++: $($bad.Count) of $($files.Count) files are not formatted:"
        foreach ($rel in $bad) { Write-PlainLog "  $rel" }
        Write-ErrorLog 'Run devkit/scripts/format.ps1 to fix them.'
        return 1
    }

    if ($bad.Count -eq 0) {
        Write-SuccessLog "C++: all $($files.Count) files were already formatted."
    }
    else {
        Write-SuccessLog "C++: reformatted $($bad.Count) of $($files.Count) files:"
        foreach ($rel in $bad) { Write-PlainLog "  $rel" }
    }
    return 0
}

# ---------------------------------------------------------------------------

if ($Check) {
    Write-Banner 'Coreverse - format check (no files will be changed)'
}
else {
    Write-Banner 'Coreverse - format'
}

$result = 0

Push-Location $RepoRoot
try {
    if ($doRust) {
        $rc = Invoke-RustFormat
        if ($rc -gt $result) { $result = $rc }
    }

    if ($doCpp) {
        $rc = Invoke-CppFormat
        if ($rc -gt $result) { $result = $rc }
    }
}
finally {
    Pop-Location
}

Write-Host ''
if ($result -eq 0) {
    if ($Check) { Write-SuccessLog 'Format check passed.' } else { Write-SuccessLog 'Done.' }
}
else {
    Write-ErrorLog "Finished with problems (exit code $result)."
}

exit $result
