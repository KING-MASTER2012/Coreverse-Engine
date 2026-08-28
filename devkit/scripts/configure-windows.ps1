#Requires -Version 7.0
<#
.SYNOPSIS
    Configures Coreverse Engine with CMake, guaranteeing an x64 MSVC
    environment regardless of which shell this was launched from.

.DESCRIPTION
    Root cause this fixes: CMakePresets.json's Windows presets use
    `"architecture": { "strategy": "external" }`, meaning CMake does not pick
    the host/target architecture itself — it inherits whatever the launching
    shell's environment already has active. A generic "Developer PowerShell
    for VS" activates x86 host tools by default; linking then mixes an x86
    link.exe with the x64 `ffi.lib` produced by the Rust/Corrosion build,
    producing LNK4272 ('x64' library machine type conflicts with 'x86' target
    machine type) and leading-underscore (`_ffi_build_info_string`) symbol
    mismatches — 32-bit cdecl name mangling, which 64-bit MSVC does not use.

    This script finds Visual Studio via vswhere, runs vcvarsall.bat x64 in a
    child cmd.exe process, imports the resulting environment (PATH, INCLUDE,
    LIB, ...) into the current PowerShell session, and only then invokes
    `cmake --preset`. The x64 environment is therefore guaranteed no matter
    which shell — Developer PowerShell, a plain PowerShell window, Windows
    Terminal, VS Code's integrated terminal — this script was started from.

.PARAMETER Preset
    The CMakePresets.json configure preset to use. Defaults to 'windows-x64'.

.PARAMETER Args
    Any additional arguments are forwarded verbatim to `cmake --preset`.

.EXAMPLE
    ./devkit/scripts/configure-windows.ps1
.EXAMPLE
    ./devkit/scripts/configure-windows.ps1 -Preset windows-x64-static -DBUILD_TESTS=ON
#>

[CmdletBinding()]
param(
    [string]$Preset = 'windows-x64',

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Args
)

$ErrorActionPreference = 'Stop'
$PSRoot = $PSScriptRoot
$RepoRoot = Resolve-Path "$PSRoot/../.."

. "$PSRoot/setup/powershell/scripts/common/logger.ps1"

Write-Banner -Title 'Coreverse - Windows x64 Configure'

# --- 1. Locate Visual Studio via vswhere ---
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) {
    Write-ErrorLog -Message "vswhere.exe not found at '$vswhere'. Is Visual Studio installed? (see devkit/scripts/setup/powershell/scripts/toolchain/check-vs.ps1)"
    exit 1
}

$installPath = (& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath) 2>$null
if (-not $installPath) {
    Write-ErrorLog -Message 'No Visual Studio installation with the VC++ Tools component was found.'
    Write-ErrorLog -Message 'Run devkit/scripts/setup/powershell/bootstrap.ps1 first, or check-vs.ps1 directly.'
    exit 1
}
Write-InfoLog -Message "Visual Studio found at: $installPath"

$vcvarsall = Join-Path $installPath 'VC\Auxiliary\Build\vcvarsall.bat'
if (-not (Test-Path $vcvarsall)) {
    Write-ErrorLog -Message "vcvarsall.bat not found at '$vcvarsall'."
    exit 1
}

# --- 2. Run vcvarsall.bat x64 in a child cmd.exe and import its environment ---
Write-InfoLog -Message 'Activating x64 MSVC environment (vcvarsall.bat x64)...'

$envDump = & cmd.exe /c "`"$vcvarsall`" x64 >nul 2>&1 && set"
if ($LASTEXITCODE -ne 0 -or -not $envDump) {
    Write-ErrorLog -Message "vcvarsall.bat x64 failed (exit code $LASTEXITCODE)."
    exit 1
}

$importedCount = 0
foreach ($line in $envDump) {
    $idx = $line.IndexOf('=')
    if ($idx -lt 1) { continue }
    $name = $line.Substring(0, $idx)
    $value = $line.Substring($idx + 1)
    # Skip cmd.exe-only pseudo-variables (e.g. "=C:", "=ExitCode").
    if ($name.StartsWith('=')) { continue }
    Set-Item -Path "Env:$name" -Value $value
    $importedCount++
}
Write-SuccessLog -Message "x64 environment imported ($importedCount variables)."

# --- 3. Sanity check: confirm the resulting link.exe is actually the x64 host tool ---
$linkCmd = Get-Command link.exe -ErrorAction SilentlyContinue
if ($linkCmd -and ($linkCmd.Source -notmatch '\\Hostx64\\x64\\')) {
    Write-WarningLog -Message "link.exe resolved to '$($linkCmd.Source)', which does not look like the Hostx64\x64 toolset. Continuing, but double-check the result if the build fails again."
} elseif ($linkCmd) {
    Write-SuccessLog -Message "link.exe confirmed as x64 host/target: $($linkCmd.Source)"
}

# --- 4. Forward to cmake --preset ---
Write-InfoLog -Message "Running: cmake --preset $Preset $($Args -join ' ')"

Push-Location $RepoRoot
try {
    & cmake --preset $Preset @Args
    if ($LASTEXITCODE -ne 0) {
        throw "cmake --preset $Preset failed (exit code $LASTEXITCODE)."
    }
    Write-SuccessLog -Message 'CMake configuration completed successfully.'
} catch {
    Write-ErrorLog -Message $_.Exception.Message
    exit 1
} finally {
    Pop-Location
}
