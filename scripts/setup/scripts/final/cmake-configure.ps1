#Requires -Version 7.0
param(
    [string]$SourceDir = '.',
    [string]$BuildDir = 'build',
    [string]$VcpkgDir = 'vendor/vcpkg',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"

$ToolName = 'CMake Configure'
$result = [PSCustomObject]@{ Tool = $ToolName; Status = 'Unknown'; Detail = $null }

$cmakeCmd = Get-Command cmake -ErrorAction SilentlyContinue
if (-not $cmakeCmd) {
    Write-ErrorLog -Message 'CMake not found on PATH. First, the toolchain phase must be completed.' -Source $ToolName
    $result.Status = 'Failed'
    return $result
}

$cmakeArgs = @('-S', $SourceDir, '-B', $BuildDir)

$toolchainFile = Join-Path $VcpkgDir 'scripts/buildsystems/vcpkg.cmake'
if (Test-Path $toolchainFile) {
    $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$toolchainFile"
    Write-InfoLog -Message "vcpkg toolchain file found, adding CMake." -Source $ToolName
}

if ($DryRun) {
    Write-InfoLog -Message "[DryRun] 'cmake $($cmakeArgs -join ' ')' was to be run." -Source $ToolName
    $result.Status = 'DryRun'
    return $result
}

Write-InfoLog -Message "'cmake $($cmakeArgs -join ' ')' running..." -Source $ToolName
& cmake @cmakeArgs 2>&1 | ForEach-Object { Write-PlainLog -Message $_ -Source $ToolName }

if ($LASTEXITCODE -eq 0) {
    $result.Status = 'OK'
    Write-SuccessLog -Message "CMake configuration completed ($BuildDir)." -Source $ToolName
} else {
    $result.Status = 'Failed'
    Write-ErrorLog -Message "CMake configure ended with error code: $LASTEXITCODE" -Source $ToolName
}

return $result
