#Requires -Version 7.0

<#
.SYNOPSIS
    Configures the project's CMake build directory.

.DESCRIPTION
    Runs the CMake configure step and automatically enables the
    project-local vcpkg toolchain when available.

.NOTES
    This script only performs the configure step.
#>

param(
    [string]$SourceDir = ".",
    [string]$BuildDir = "build",
    [string]$VcpkgRoot = "vendor/vcpkg",
    [string]$InstalledDir = "vcpkg_installed",
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"

$ToolName = "CMake Configure"

$result = [PSCustomObject]@{
    Tool   = $ToolName
    Status = "Unknown"
    Detail = $null
}

$cmake = Get-Command cmake -ErrorAction SilentlyContinue

if (-not $cmake) {
    Write-ErrorLog `
        -Source $ToolName `
        -Message "CMake was not found on PATH."

    $result.Status = "Failed"
    return $result
}

try {

    # Resolve all paths to absolute paths.
    # Using [System.IO.Path]::GetFullPath prevents errors if directories like 'build' do not exist yet.

    $SourceDir    = [System.IO.Path]::GetFullPath($SourceDir)
    $BuildDir     = [System.IO.Path]::GetFullPath($BuildDir)
    $VcpkgRoot    = [System.IO.Path]::GetFullPath($VcpkgRoot)
    $InstalledDir = [System.IO.Path]::GetFullPath($InstalledDir)

    # Ensure build directory is created before invoking CMake
    if (-not (Test-Path $BuildDir)) {
        Write-InfoLog -Source $ToolName -Message "Creating build directory at '$BuildDir'..."
        New-Item -ItemType Directory -Path $BuildDir -Force | Out-Null
    }

} catch {
    Write-ErrorLog `
        -Source $ToolName `
        -Message "Failed to resolve or create project paths. $($_.Exception.Message)"

    $result.Status = "Failed"
    return $result
}

$toolchainFile = Join-Path $VcpkgRoot "scripts/buildsystems/vcpkg.cmake"

Write-InfoLog -Source $ToolName -Message "Source directory   : $SourceDir"
Write-InfoLog -Source $ToolName -Message "Build directory    : $BuildDir"
Write-InfoLog -Source $ToolName -Message "vcpkg root         : $VcpkgRoot"
Write-InfoLog -Source $ToolName -Message "Installed packages : $InstalledDir"
Write-InfoLog -Source $ToolName -Message "Toolchain file     : $toolchainFile"

$cmakeArgs = @(
    "-S", $SourceDir,
    "-B", $BuildDir
)

if (Test-Path $toolchainFile) {

    Write-InfoLog `
        -Source $ToolName `
        -Message "Using the project-local vcpkg toolchain."

    $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$toolchainFile"
    $cmakeArgs += "-DVCPKG_INSTALLED_DIR=$InstalledDir"

} else {

    Write-ErrorLog `
        -Source $ToolName `
        -Message "The vcpkg toolchain file was not found:`n$toolchainFile"

    $result.Status = "Failed"
    return $result

}

if ($DryRun) {

    Write-InfoLog `
        -Source $ToolName `
        -Message "[DryRun] cmake $($cmakeArgs -join ' ')"

    $result.Status = "DryRun"
    return $result

}

try {

    Write-InfoLog `
        -Source $ToolName `
        -Message "Running CMake configure..."

    & cmake @cmakeArgs 2>&1 |
        ForEach-Object {
            if (-not [string]::IsNullOrWhiteSpace($_)) {
                Write-PlainLog `
                    -Source $ToolName `
                    -Message $_
            }
        }

    if ($LASTEXITCODE -ne 0) {
        throw "CMake exited with code $LASTEXITCODE."
    }

    $result.Status = "OK"

    Write-SuccessLog `
        -Source $ToolName `
        -Message "CMake configuration completed successfully."

} catch {

    $result.Status = "Failed"

    Write-ErrorLog `
        -Source $ToolName `
        -Message "CMake configuration failed. $($_.Exception.Message)"

}

return $result
