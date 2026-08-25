#Requires -Version 7.0
<#
.SYNOPSIS
    Coreverse Bootstrap - Windows entry point.
.DESCRIPTION
    Toolchain inspection/installation, project dependencies installation and CMake configuration
    automated by a single command. Independent tools run in parallel, dependents (Rustup->Cargo,
    Git->vcpkg) run inline.
.PARAMETER Yes
    Unapproved/unattended mode. (Currently, the script doesn't require interactive approval;
    it's reserved for possible approval steps to be added in the future.)
.PARAMETER DryRun
    Simply log what happens without making any installations or changes.
.PARAMETER SkipElevation
    For testing purposes: skip automatic administrator privilege escalation.
.EXAMPLE
    ./bootstrap.ps1
.EXAMPLE
    ./bootstrap.ps1 -DryRun
.EXAMPLE
    ./bootstrap.ps1 -Yes
#>
[CmdletBinding()]
param(
    [switch]$Yes,
    [switch]$DryRun,
    [switch]$SkipElevation
)

$ErrorActionPreference = 'Stop'
$PSRoot = $PSScriptRoot
$ScriptsRoot = Resolve-Path "$PSScriptRoot\..\"

. "$PSRoot/scripts/common/logger.ps1"
. "$PSRoot/scripts/common/os-detect.ps1"
. "$PSRoot/scripts/common/parallel-runner.ps1"
. "$PSRoot/scripts/final/summary-table.ps1"

Write-Banner -Title 'Coreverse Bootstrap (Windows)'

# --- 0. PowerShell Version Inspection ---
# Invoke-TaskGraph / Invoke-ParallelTasks uses ForEach-Object -Parallel, which requires PS 7.0+.
if (-not (Test-MinimumPSVersion -MinimumMajor 7)) {
    Write-ErrorLog -Message "PowerShell 7 or later is required. Current: $($PSVersionTable.PSVersion) ($($PSVersionTable.PSEdition))."
    Write-ErrorLog -Message 'For installation: https://aka.ms/powershell-release?tag=stable'
    exit 1
}

# --- 1. Environment Info ---
$osInfo = Get-OSInfo
Write-InfoLog -Message "$($osInfo.Caption) | $($osInfo.Architecture) | PowerShell $($osInfo.PSVersion) ($($osInfo.PSEdition))"

# --- 2. Administrator Privileges (one-time elevation, then silently resume) ---
if (-not $osInfo.IsAdmin -and -not $SkipElevation -and -not $env:COREVERSE_BOOTSTRAP_ELEVATED) {
    Write-WarningLog -Message "Administrator privileges are required; restarting with elevated session..."

    # Ensure the new elevated window starts in the correct directory and stays open on failure.
    $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    if ($Yes)    { $argList += '-Yes' }
    if ($DryRun) { $argList += '-DryRun' }

    $env:COREVERSE_BOOTSTRAP_ELEVATED = '1'
    try {
        Start-Process -FilePath 'pwsh' -ArgumentList $argList -WorkingDirectory $PWD -Verb RunAs -Wait
        exit $LASTEXITCODE
    } catch {
        Write-ErrorLog -Message "The elevated session could not be started: $($_.Exception.Message)"
        Write-ErrorLog -Message 'Please run this script manually in an administrator PowerShell window.'
        exit 1
    }
}

if ($osInfo.IsAdmin) {
    Write-SuccessLog -Message 'Running with administrator privileges.'
} else {
    Write-WarningLog -Message 'Continuing without administrator privileges (-SkipElevation). Some installations may fail.'
}

if ($DryRun) {
    Write-WarningLog -Message 'DRY-RUN mode enabled: no installations or modifications will be made.'
}

# --- 3. Configuration Loading & Path Resolution ---
$toolVersions = Get-Content "$ScriptsRoot/config/tool-versions.json" -Raw | ConvertFrom-Json
$projectPaths = Get-Content "$ScriptsRoot/config/project-paths.json" -Raw | ConvertFrom-Json

$commonArgs = @{ DryRun = [bool]$DryRun }

# Centralized Absolute Path Resolution
# Calculate EngineRoot by going 4 levels up from coreverse-engine/devkit/scripts/setup/powershell/
$EngineRoot = (Resolve-Path "$PSRoot\..\..\..\..\").Path

function ConvertTo-AbsolutePath {
    param([string]$RelPath)

    if ([string]::IsNullOrWhiteSpace($RelPath)) {
        return $null
    }

    if ([System.IO.Path]::IsPathRooted($RelPath)) {
        return $RelPath
    }

    return [System.IO.Path]::GetFullPath((Join-Path $EngineRoot $RelPath))
}

Write-InfoLog -Message "Resolving project paths relative to Engine Root: $EngineRoot"

$absVcpkgRoot         = ConvertTo-AbsolutePath $projectPaths.vcpkgRoot
$absVcpkgManifestDir  = ConvertTo-AbsolutePath $projectPaths.vcpkgManifestDir
$absVcpkgInstalledDir = ConvertTo-AbsolutePath $projectPaths.vcpkgInstalledDir
$absCMakeSourceDir    = ConvertTo-AbsolutePath $projectPaths.cmakeSourceDir
$absCMakeBuildDir     = ConvertTo-AbsolutePath $projectPaths.cmakeBuildDir
$absCargoWorkspace    = ConvertTo-AbsolutePath $projectPaths.cargoWorkspaceRoot

function ConvertTo-FlatResults {

    param(
        [array]$GraphResults
    )

    $flat = @()

    foreach ($r in $GraphResults) {

        if (-not $r.Success) {

            Write-ErrorLog -Message "$($r.Name): $($r.Error)"

            $flat += [PSCustomObject]@{
                Tool         = $r.Name
                FinalVersion = $null
                Status       = 'Failed'
            }

            continue
        }

        if ($null -eq $r.Result) {

            Write-ErrorLog -Message "$($r.Name): No result returned."

            $flat += [PSCustomObject]@{
                Tool         = $r.Name
                FinalVersion = $null
                Status       = 'Failed'
            }

            continue
        }

        $flat += $r.Result
    }

    return $flat
}

# --- 4. Phase 1: Toolchain Inspection (Dependency graph, parallel) ---
Write-Banner -Title '1/3 - Toolchain Inspection'

$toolchainDir = "$PSRoot/scripts/toolchain"

$toolchainTasks = @(
    @{ Name = 'Rustup';     ScriptPath = "$toolchainDir/check-rustup.ps1"; DependsOn = @();         Arguments = (@{ RequiredVersion = $toolVersions.rustup.minVersion } + $commonArgs) }
    @{ Name = 'Cargo';      ScriptPath = "$toolchainDir/check-cargo.ps1";  DependsOn = @('Rustup');  Arguments = (@{ RequiredVersion = $toolVersions.cargo.minVersion } + $commonArgs) }
    @{ Name = 'Git';        ScriptPath = "$toolchainDir/check-git.ps1";    DependsOn = @();          Arguments = (@{ RequiredVersion = $toolVersions.git.minVersion } + $commonArgs) }
    @{ Name = 'LLVM/Clang'; ScriptPath = "$toolchainDir/check-llvm.ps1";   DependsOn = @();          Arguments = (@{ RequiredVersion = $toolVersions.llvm.minVersion } + $commonArgs) }
    @{ Name = 'CMake';      ScriptPath = "$toolchainDir/check-cmake.ps1";  DependsOn = @();          Arguments = (@{ RequiredVersion = $toolVersions.cmake.minVersion } + $commonArgs) }
    @{ Name = 'Ninja';      ScriptPath = "$toolchainDir/check-ninja.ps1";  DependsOn = @();          Arguments = (@{ RequiredVersion = $toolVersions.ninja.minVersion } + $commonArgs) }
    @{ Name = 'vcpkg';      ScriptPath = "$toolchainDir/check-vcpkg.ps1";  DependsOn = @('Git');     Arguments = (@{ VcpkgRoot = $absVcpkgRoot } + $commonArgs) }
)

$toolchainGraphResults = Invoke-TaskGraph -Tasks $toolchainTasks
$toolchainFlat = ConvertTo-FlatResults -GraphResults $toolchainGraphResults

# --- 5. Phase 2: Project Dependencies (Independent package managers, parallel) ---
Write-Banner -Title '2/3 - Project Dependencies'

$depDir = "$PSRoot/scripts/dependencies"

$depTasks = @(
    @{ Name = 'Cargo Deps'; ScriptPath = "$depDir/parse-cargo.ps1"; Arguments = (@{ WorkspaceRoot = $absCargoWorkspace } + $commonArgs) }
    @{ Name = 'vcpkg Deps'; ScriptPath = "$depDir/parse-vcpkg.ps1"; Arguments = (@{ ManifestDir = $absVcpkgManifestDir; VcpkgRoot = $absVcpkgRoot; InstalledDir = $absVcpkgInstalledDir} + $commonArgs) }
)

$depGraphResults = Invoke-ParallelTasks -Tasks $depTasks
$depFlat = ConvertTo-FlatResults -GraphResults $depGraphResults

# --- 6. Phase 3: CMake Configuration ---
Write-Banner -Title '3/3 - CMake Configuration'

$cmakeConfigureArgs = @{
    SourceDir    = $absCMakeSourceDir
    BuildDir     = $absCMakeBuildDir
    VcpkgRoot    = $absVcpkgRoot
    InstalledDir = $absVcpkgInstalledDir
} + $commonArgs

$cmakeResult = & "$PSRoot/scripts/final/cmake-configure.ps1" @cmakeConfigureArgs

# --- 7. Summary Table and Exit Code ---
$allResults = @()
$allResults += $toolchainFlat
$allResults += $depFlat
$allResults += $cmakeResult

$summary = Show-SummaryTable -Results $allResults

if ($summary.FailedCount -gt 0) {
    exit 1
} else {
    exit 0
}
