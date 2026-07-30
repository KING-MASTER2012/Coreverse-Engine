#Requires -Version 7.0
param(
    [string]$ManifestDir = '.',
    [string]$VcpkgDir = 'vendor/vcpkg',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"

$ToolName = 'vcpkg Deps'
$result = [PSCustomObject]@{ Tool = $ToolName; Status = 'Unknown'; Detail = $null }

$manifestPath = Join-Path $ManifestDir 'vcpkg.json'
if (-not (Test-Path $manifestPath)) {
    Write-WarningLog -Message "vcpkg.json not found ($manifestPath), this step skipped." -Source $ToolName
    $result.Status = 'Skipped'
    return $result
}

if ($DryRun) {
    Write-InfoLog -Message "[DryRun] For '$ManifestDir', manifest-mode 'vcpkg install' was to be run." -Source $ToolName
    $result.Status = 'DryRun'
    return $result
}

$vcpkgExe = Join-Path $VcpkgDir 'vcpkg.exe'
if (-not (Test-Path $vcpkgExe)) {
    Write-ErrorLog -Message "vcpkg.exe not found ($vcpkgExe). First, the toolchain phase(check-vcpkg) must be completed." -Source $ToolName
    $result.Status = 'Failed'
    return $result
}

try {
    Write-InfoLog -Message 'vcpkg install (manifest mode) running...' -Source $ToolName
    & $vcpkgExe install "--x-manifest-root=$ManifestDir" 2>&1 | ForEach-Object { Write-PlainLog -Message $_ -Source $ToolName }

    if ($LASTEXITCODE -eq 0) {
        $result.Status = 'OK'
        Write-SuccessLog -Message 'C++ dependencies (vcpkg) downloaded.' -Source $ToolName
    } else {
        $result.Status = 'Failed'
        Write-ErrorLog -Message "'vcpkg install' ended with error code: $LASTEXITCODE" -Source $ToolName
    }
} catch {
    $result.Status = 'Failed'
    Write-ErrorLog -Message "'vcpkg install' unsuccessful: $($_.Exception.Message)" -Source $ToolName
}

return $result
