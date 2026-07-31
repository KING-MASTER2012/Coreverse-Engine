#Requires -Version 7.0

<#
.SYNOPSIS
    Ensures that the project-local vcpkg submodule exists and is initialized.

.DESCRIPTION
    vcpkg is managed as a Git Submodule inside the project.
    This script ensures the submodule is initialized and checked out to the
    correct commit specified by the main repository, then bootstraps it.

.NOTES
    Git must already be available on PATH.
#>

param(
    [string]$VcpkgRoot = "vcpkg",
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"

$ToolName = "vcpkg"

$result = [PSCustomObject]@{
    Tool            = $ToolName
    PreviousVersion = $null
    RequiredVersion = "Submodule-Locked"
    FinalVersion    = $null
    Source          = "Git-Submodule"
    Status          = "Unknown"
}

$git = Get-Command git -ErrorAction SilentlyContinue

if (-not $git) {
    $result.Status = "Failed"
    Write-ErrorLog `
        -Source $ToolName `
        -Message "Git was not found on PATH. vcpkg requires Git to initialize its submodule."

    return $result
}

# Ensure path is absolute
$VcpkgRoot = [System.IO.Path]::GetFullPath($VcpkgRoot)

$vcpkgExe = Join-Path $VcpkgRoot "vcpkg.exe"
$bootstrapScript = Join-Path $VcpkgRoot "bootstrap-vcpkg.bat"
$gitPath = Join-Path $VcpkgRoot ".git"

# In a submodule, .git is a file, not a directory. Test-Path handles both.
$isInitialized = (Test-Path $gitPath) -and (Test-Path $bootstrapScript)

if ($isInitialized -and (Test-Path $vcpkgExe)) {
    $result.PreviousVersion = (& $vcpkgExe version) 2>&1 | Select-Object -First 1
    Write-InfoLog `
        -Source $ToolName `
        -Message "vcpkg submodule is already present at '$VcpkgRoot'."
} else {
    Write-WarningLog `
        -Source $ToolName `
        -Message "vcpkg submodule is not initialized or bootstrapped. Starting setup..."
}

if ($DryRun) {
    Write-InfoLog `
        -Source $ToolName `
        -Message "[DryRun] Planned operation: Initialize Git submodule and bootstrap vcpkg."
    $result.Status = "DryRun"
    return $result
}

try {
    Write-InfoLog `
        -Source $ToolName `
        -Message "Synchronizing vcpkg submodule with the main repository..."

    # Initialize and update the submodule.
    # This locks vcpkg to the commit registered in your main Git repository.
    & git submodule update --init --recursive $VcpkgRoot 2>&1 | Out-Null

    if (-not (Test-Path $bootstrapScript)) {
        throw "Failed to initialize the vcpkg submodule. Ensure '.gitmodules' exists in the repository root."
    }

    Write-InfoLog `
        -Source $ToolName `
        -Message "Bootstrapping vcpkg..."

    & $bootstrapScript -disableMetrics 2>&1 | Out-Null

    if (-not (Test-Path $vcpkgExe)) {
        throw "vcpkg.exe was not generated after bootstrapping."
    }

    $result.FinalVersion = (& $vcpkgExe version) 2>&1 | Select-Object -First 1
    $result.Status = if ($isInitialized) { "Updated" } else { "Installed" }

    Write-SuccessLog `
        -Source $ToolName `
        -Message "vcpkg submodule is ready. Version: $($result.FinalVersion)"

} catch {
    $result.Status = "Failed"
    Write-ErrorLog `
        -Source $ToolName `
        -Message "Failed to prepare vcpkg submodule. $($_.Exception.Message)"
}

return $result
