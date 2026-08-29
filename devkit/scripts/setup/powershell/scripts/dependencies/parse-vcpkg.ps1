#Requires -Version 7.0

<#
.SYNOPSIS
    Restores project C++ dependencies using vcpkg manifest mode.

.DESCRIPTION
    Executes 'vcpkg install' using the project's vcpkg.json manifest.

    The vcpkg repository and the installed package directory are treated as
    separate locations.

.NOTES
    This script assumes that check-vcpkg.ps1 has already completed
    successfully.
#>

param(
    [string]$ManifestDir = ".",
    [string]$VcpkgRoot = "third_party/vcpkg",
    [string]$InstalledDir = "vcpkg_installed",
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"

$ToolName = "vcpkg Dependencies"

$result = [PSCustomObject]@{
    Tool   = $ToolName
    Status = "Unknown"
    Detail = $null
}

# Ensure all paths are absolute
$ManifestDir  = [System.IO.Path]::GetFullPath($ManifestDir)
$VcpkgRoot    = [System.IO.Path]::GetFullPath($VcpkgRoot)
$InstalledDir = [System.IO.Path]::GetFullPath($InstalledDir)

$manifestFile = Join-Path $ManifestDir "vcpkg.json"

if (-not (Test-Path $manifestFile)) {

    Write-WarningLog `
        -Source $ToolName `
        -Message "No vcpkg.json manifest was found at '$manifestFile'. Skipping dependency restoration."

    $result.Status = "Skipped"
    return $result
}

$vcpkgExe = Join-Path $VcpkgRoot "vcpkg.exe"

if (-not (Test-Path $vcpkgExe)) {

    Write-ErrorLog `
        -Source $ToolName `
        -Message "vcpkg.exe was not found at '$vcpkgExe'. Run the toolchain phase before restoring dependencies."

    $result.Status = "Failed"
    return $result
}

if ($DryRun) {

    Write-InfoLog `
        -Source $ToolName `
        -Message "[DryRun] Planned operation: restore C++ dependencies using manifest mode."

    $result.Status = "DryRun"
    return $result
}

try {

    Write-InfoLog `
        -Source $ToolName `
        -Message "Restoring C++ dependencies using vcpkg manifest mode..."

    & $vcpkgExe install `
        "--x-manifest-root=$ManifestDir" `
        "--x-install-root=$InstalledDir" 2>&1 |
        ForEach-Object {

            # Ignore empty/null output entries.
            if ($null -ne $_) {
                $message = $_.ToString()

                if (-not [string]::IsNullOrWhiteSpace($message)) {
                    Write-PlainLog `
                        -Source $ToolName `
                        -Message $message
                }
            }
        }

    $vcpkgExitCode = $LASTEXITCODE

    if ($vcpkgExitCode -ne 0) {
        throw "'vcpkg install' exited with code $vcpkgExitCode."
    }

    $result.Status = "OK"
    $result.Detail = "C++ dependencies restored successfully."

    Write-SuccessLog `
        -Source $ToolName `
        -Message "All C++ dependencies were restored successfully."

} catch {

    $result.Status = "Failed"

    $exceptionMessage = $_.Exception.Message

    if ([string]::IsNullOrWhiteSpace($exceptionMessage)) {
        $exceptionMessage = "Unknown error while restoring C++ dependencies."
    }

    $result.Detail = $exceptionMessage

    Write-ErrorLog `
        -Source $ToolName `
        -Message "Failed to restore C++ dependencies. $exceptionMessage"
}

return $result
