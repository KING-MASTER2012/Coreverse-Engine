#Requires -Version 7.0

<#
.SYNOPSIS
    Ensures that the project-local vcpkg submodule exists and is initialized.
#>

param(
    [string]$VcpkgRoot = "third_party/vcpkg",
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
    Write-ErrorLog -Source $ToolName -Message "Git was not found on PATH. vcpkg requires Git to initialize its submodule."
    return $result
}

$VcpkgRoot = [System.IO.Path]::GetFullPath($VcpkgRoot)
$vcpkgExe = Join-Path $VcpkgRoot "vcpkg.exe"
$bootstrapScript = Join-Path $VcpkgRoot "bootstrap-vcpkg.bat"
$gitPath = Join-Path $VcpkgRoot ".git"

$isInitialized = (Test-Path $gitPath) -and (Test-Path $bootstrapScript)

if ($isInitialized -and (Test-Path $vcpkgExe)) {
    # Using -join to ensure single string even if exe returns multi-line output
    $result.PreviousVersion = (& $vcpkgExe version 2>&1 | Select-Object -First 1) -join ''
    Write-InfoLog -Source $ToolName -Message "vcpkg submodule is already present at '$VcpkgRoot'."
} else {
    Write-WarningLog -Source $ToolName -Message "vcpkg submodule is not initialized or bootstrapped. Starting setup..."
}

if ($DryRun) {
    Write-InfoLog -Source $ToolName -Message "[DryRun] Planned operation: Initialize Git submodule and bootstrap vcpkg."
    $result.Status = "DryRun"
    return $result
}

try {
    Write-InfoLog -Source $ToolName -Message "Synchronizing vcpkg submodule with the main repository..."

    $parentDir = Split-Path -Parent $VcpkgRoot
    $moduleName = Split-Path -Leaf $VcpkgRoot

    if (-not (Test-Path $parentDir)) {
        New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
    }

    # FIX: Native executables in PS Runspaces (Parallel-Runner) ignore Push-Location.
    # We must pass the working directory directly to Git using the -C flag.
    & git -C $parentDir submodule update --init --recursive $moduleName 2>&1 | Out-Null

    if (-not (Test-Path $bootstrapScript)) {
        throw "Failed to initialize the vcpkg submodule. Ensure '.gitmodules' is correctly configured."
    }

    Write-InfoLog -Source $ToolName -Message "Bootstrapping vcpkg..."
    & $bootstrapScript -disableMetrics 2>&1 | Out-Null

    if (-not (Test-Path $vcpkgExe)) {
        throw "vcpkg.exe was not generated after bootstrapping."
    }

    $result.FinalVersion = (& $vcpkgExe version 2>&1 | Select-Object -First 1) -join ''
    $result.Status = if ($isInitialized) { "Updated" } else { "Installed" }

    Write-SuccessLog -Source $ToolName -Message "vcpkg submodule is ready. Version: $($result.FinalVersion)"

} catch {
    $result.Status = "Failed"
    Write-ErrorLog -Source $ToolName -Message "Failed to prepare vcpkg submodule. $($_.Exception.Message)"
}

$result
