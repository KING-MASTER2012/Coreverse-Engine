#Requires -Version 7.0
<#
.SYNOPSIS
   vcpkg is installed locally to the project, not system-wide (vendor/vcpkg, manifest mode).
   Therefore, instead of the "is it in the PATH" logic like other tools, it uses the "is it in the project folder,
   is it up-to-date" logic. Git must be installed beforehand (DependsOn = @('Git')).
   NOTES
   This script does not use the generic Invoke-ToolCheck helper; vcpkg's version concept
   (commit-based, "always the most up-to-date" policy) does not conform to semver comparison.
#>
param(
    [string]$VcpkgDir = 'vendor/vcpkg',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"

$ToolName = 'vcpkg'
$result = [PSCustomObject]@{
    Tool            = $ToolName
    PreviousVersion = $null
    RequiredVersion = 'latest'
    FinalVersion    = $null
    Source          = $null
    Status          = 'Unknown'
}

$gitCmd = Get-Command git -ErrorAction SilentlyContinue
if (-not $gitCmd) {
    $result.Status = 'Failed'
    Write-ErrorLog -Message 'Git not found. Git is necessary for vcpkg clone process' -Source $ToolName
    return $result
}

$vcpkgExe = Join-Path $VcpkgDir 'vcpkg.exe'
$alreadyCloned = Test-Path $vcpkgExe

if ($alreadyCloned) {
    $result.PreviousVersion = (& $vcpkgExe version) 2>&1 | Select-Object -First 1
    Write-InfoLog -Message "Found: $VcpkgDir ($($result.PreviousVersion))" -Source $ToolName
} else {
    Write-WarningLog -Message "$VcpkgDir not found, the initial setup will be done." -Source $ToolName
}

if ($DryRun) {
    $action = if ($alreadyCloned) { 'git pull + re-bootstrap' } else { 'clone + bootstrap' }
    Write-InfoLog -Message "[DryRun] $action was to be done." -Source $ToolName
    $result.Status = 'DryRun'
    return $result
}

try {
    if (-not $alreadyCloned) {
        $parentDir = Split-Path -Parent $VcpkgDir
        if ($parentDir -and -not (Test-Path $parentDir)) {
            New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
        }
        Write-InfoLog -Message 'https://github.com/microsoft/vcpkg.git cloning...' -Source $ToolName
        & git clone --depth 1 'https://github.com/microsoft/vcpkg.git' $VcpkgDir 2>&1 | Out-Null
    } else {
        Write-InfoLog -Message 'Current vcpkg updating (git pull)...' -Source $ToolName
        Push-Location $VcpkgDir
        try {
            & git pull --ff-only 2>&1 | Out-Null
        } finally {
            Pop-Location
        }
    }

    Write-InfoLog -Message 'bootstrap-vcpkg.bat running...' -Source $ToolName
    $bootstrapScript = Join-Path $VcpkgDir 'bootstrap-vcpkg.bat'
    & $bootstrapScript -disableMetrics 2>&1 | Out-Null

    if (Test-Path $vcpkgExe) {
        $result.FinalVersion = (& $vcpkgExe version) 2>&1 | Select-Object -First 1
        $result.Source = if ($alreadyCloned) { 'ProjectLocal-Updated' } else { 'ProjectLocal-Cloned' }
        $result.Status = if ($alreadyCloned) { 'Upgraded' } else { 'Installed' }
        Write-SuccessLog -Message "Hazir: $($result.FinalVersion)" -Source $ToolName
    } else {
        $result.Status = 'Failed'
        Write-ErrorLog -Message 'vcpkg.exe not found after bootstrap.' -Source $ToolName
    }
} catch {
    $result.Status = 'Failed'
    Write-ErrorLog -Message "vcpkg installation unsuccesful: $($_.Exception.Message)" -Source $ToolName
}

return $result
