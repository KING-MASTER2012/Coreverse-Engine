#Requires -Version 7.0
<#
.SYNOPSIS
    Coreverse Bootstrap - Windows entry point.
.DESCRIPTION
    Toolchain inspection/installlation, project dependencies installation and CMake configuration
    automates by only one command. Independent tools parallel, dependants (Rustup->Cargo,
    Node->npm, Git->vcpkg) runs as in-line.
.PARAMETER Yes
    Unapproved/unattended mod. (Currently, the script doesn't require interactive approval;
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
$RepoRoot = $PSScriptRoot

. "$RepoRoot/scripts/common/logger.ps1"
. "$RepoRoot/scripts/common/os-detect.ps1"
. "$RepoRoot/scripts/common/parallel-runner.ps1"
. "$RepoRoot/scripts/final/summary-table.ps1"

Write-Banner -Title 'CoreVerse Bootstrap (Windows)'

# --- 0. PowerShell surum kontrolu ---
# Invoke-TaskGraph / Invoke-ParallelTasks uses ForEach-Object -Parallel, this requires PS 7.0+.
if (-not (Test-MinimumPSVersion -MinimumMajor 7)) {
    Write-ErrorLog -Message "PowerShell 7 or later is required. Now: $($PSVersionTable.PSVersion) ($($PSVersionTable.PSEdition))."
    Write-ErrorLog -Message 'For installation: https://aka.ms/powershell-release?tag=stable'
    exit 1
}

# --- 1. Environment Info ---
$osInfo = Get-OSInfo
Write-InfoLog -Message "$($osInfo.Caption) | $($osInfo.Architecture) | PowerShell $($osInfo.PSVersion) ($($osInfo.PSEdition))"

# --- 2. Administrator privileges (one-time upgrade, then silently resume) ---
if (-not $osInfo.IsAdmin -and -not $SkipElevation -and -not $env:COREVERSE_BOOTSTRAP_ELEVATED) {
    Write-WarningLog -Message "Administrator privileges are required; it's restarting with an elevated session..."

    $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    if ($Yes)    { $argList += '-Yes' }
    if ($DryRun) { $argList += '-DryRun' }

    $env:COREVERSE_BOOTSTRAP_ELEVATED = '1'
    try {
        Start-Process -FilePath 'pwsh' -ArgumentList $argList -Verb RunAs -Wait
        exit $LASTEXITCODE
    } catch {
        Write-ErrorLog -Message "The elevated session could not be started: $($_.Exception.Message)"
        Write-ErrorLog -Message 'Please run this script manually in an administrator PowerShell window.'
        exit 1
    }
}

if ($osInfo.IsAdmin) {
    Write-SuccessLog -Message 'The work is done under managerial authority.'
} else {
    Write-WarningLog -Message 'Continuing without administrator privileges (-Skip Elevation). Some installations may fail.'
}

if ($DryRun) {
    Write-WarningLog -Message 'DRY-RUN mod enabled: no installations or modifications will be made.'
}

# --- 3. Config Uploading ---
$toolVersions = Get-Content "$RepoRoot/config/tool-versions.json" -Raw | ConvertFrom-Json
$projectPaths = Get-Content "$RepoRoot/config/project-paths.json" -Raw | ConvertFrom-Json

$commonArgs = @{ DryRun = [bool]$DryRun }

function ConvertTo-FlatResults {
    <#
        Converts the output (Name/Success/Result/Error) of Invoke-TaskGraph / Invoke-ParallelTasks
        to a flat {Tool;FinalVersion;Status} list as expected by Show-SummaryTable.
        $r.Result can sometimes be an array on its own (like parse-npm/parse-go); we flatten it
        at a single level with +=.
    #>
    param([array]$GraphResults)
    $flat = @()
    foreach ($r in $GraphResults) {
        if ($r.Success -and $null -ne $r.Result) {
            $flat += $r.Result
        } elseif (-not $r.Success) {
            $flat += [PSCustomObject]@{ Tool = $r.Name; FinalVersion = $null; Status = 'Failed'; Detail = $r.Error }
        }
    }
    return $flat
}

# --- 4. Phase 1: Toolchain inspection (with dependency graphic, parallel) ---
Write-Banner -Title '1/3 - Toolchain Inspection'

$toolchainDir = "$RepoRoot/scripts/toolchain"

$toolchainTasks = @(
    @{ Name = 'Rustup';     ScriptPath = "$toolchainDir/check-rustup.ps1"; DependsOn = @();         Arguments = (@{ RequiredVersion = $toolVersions.rustup.minVersion } + $commonArgs) }
    @{ Name = 'Cargo';      ScriptPath = "$toolchainDir/check-cargo.ps1";  DependsOn = @('Rustup');  Arguments = (@{ RequiredVersion = $toolVersions.cargo.minVersion } + $commonArgs) }
    @{ Name = 'Git';        ScriptPath = "$toolchainDir/check-git.ps1";    DependsOn = @();          Arguments = (@{ RequiredVersion = $toolVersions.git.minVersion } + $commonArgs) }
    @{ Name = 'LLVM/Clang'; ScriptPath = "$toolchainDir/check-llvm.ps1";   DependsOn = @();          Arguments = (@{ RequiredVersion = $toolVersions.llvm.minVersion } + $commonArgs) }
    @{ Name = 'CMake';      ScriptPath = "$toolchainDir/check-cmake.ps1";  DependsOn = @();          Arguments = (@{ RequiredVersion = $toolVersions.cmake.minVersion } + $commonArgs) }
    @{ Name = 'Ninja';      ScriptPath = "$toolchainDir/check-ninja.ps1"; DependsOn = @();           Arguments = (@{ RequiredVersion = $toolVersions.ninja.minVersion } + $commonArgs) }
    @{ Name = 'Go';         ScriptPath = "$toolchainDir/check-go.ps1";     DependsOn = @();          Arguments = (@{ RequiredVersion = $toolVersions.go.minVersion } + $commonArgs) }
    @{ Name = 'Node.js';    ScriptPath = "$toolchainDir/check-node.ps1";   DependsOn = @();          Arguments = (@{ RequiredVersion = $toolVersions.node.minVersion } + $commonArgs) }
    @{ Name = 'npm';        ScriptPath = "$toolchainDir/check-npm.ps1";    DependsOn = @('Node.js'); Arguments = (@{ RequiredVersion = $toolVersions.npm.minVersion } + $commonArgs) }
    @{ Name = 'vcpkg';      ScriptPath = "$toolchainDir/check-vcpkg.ps1";  DependsOn = @('Git');     Arguments = (@{ VcpkgDir = $projectPaths.vcpkgInstallDir } + $commonArgs) }
)

$toolchainGraphResults = Invoke-TaskGraph -Tasks $toolchainTasks
$toolchainFlat = ConvertTo-FlatResults -GraphResults $toolchainGraphResults

# --- 5. Phase 2: Project dependencies (independent package managers, parallel) ---
Write-Banner -Title '2/3 - Project Dependencies'

$depDir = "$RepoRoot/scripts/dependencies"

$depTasks = @(
    @{ Name = 'Cargo Deps'; ScriptPath = "$depDir/parse-cargo.ps1"; Arguments = (@{ WorkspaceRoot = $projectPaths.cargoWorkspaceRoot } + $commonArgs) }
    @{ Name = 'npm Deps';   ScriptPath = "$depDir/parse-npm.ps1";   Arguments = (@{ Projects = $projectPaths.npmProjects } + $commonArgs) }
    @{ Name = 'Go Deps';    ScriptPath = "$depDir/parse-go.ps1";    Arguments = (@{ Modules = $projectPaths.goModules } + $commonArgs) }
    @{ Name = 'vcpkg Deps'; ScriptPath = "$depDir/parse-vcpkg.ps1"; Arguments = (@{ ManifestDir = $projectPaths.vcpkgManifestDir; VcpkgDir = $projectPaths.vcpkgInstallDir } + $commonArgs) }
)

$depGraphResults = Invoke-ParallelTasks -Tasks $depTasks
$depFlat = ConvertTo-FlatResults -GraphResults $depGraphResults

# --- 6. Phase 3: CMake configuration ---
Write-Banner -Title '3/3 - CMake Configuration'

$cmakeConfigureArgs = @{
    SourceDir = $projectPaths.cmakeSourceDir
    BuildDir  = $projectPaths.cmakeBuildDir
    VcpkgDir  = $projectPaths.vcpkgInstallDir
} + $commonArgs

$cmakeResult = & "$RepoRoot/scripts/final/cmake-configure.ps1" @cmakeConfigureArgs

# --- 7. Summary table and exit code ---
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
