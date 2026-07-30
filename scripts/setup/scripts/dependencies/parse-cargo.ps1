#Requires -Version 7.0
param(
    [string]$WorkspaceRoot = '.',
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"

$ToolName = 'Cargo Deps'
$result = [PSCustomObject]@{ Tool = $ToolName; Status = 'Unknown'; Detail = $null }

$cargoTomlPath = Join-Path $WorkspaceRoot 'Cargo.toml'
if (-not (Test-Path $cargoTomlPath)) {
    Write-WarningLog -Message "Cargo.toml not found ($cargoTomlPath), this step skipped." -Source $ToolName
    $result.Status = 'Skipped'
    return $result
}

if ($DryRun) {
    Write-InfoLog -Message "[DryRun] In '$WorkspaceRoot', 'cargo fetch' was to be run." -Source $ToolName
    $result.Status = 'DryRun'
    return $result
}

$cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargoCmd) {
    Write-ErrorLog -Message 'cargo not found on PATH. First, the toolchain phase must be completed.' -Source $ToolName
    $result.Status = 'Failed'
    return $result
}

Push-Location $WorkspaceRoot
try {
    $lockExists = Test-Path 'Cargo.lock'
    $fetchArgs = if ($lockExists) { @('fetch', '--locked') } else { @('fetch') }

    Write-InfoLog -Message "cargo $($fetchArgs -join ' ') running..." -Source $ToolName
    & cargo @fetchArgs 2>&1 | ForEach-Object { Write-PlainLog -Message $_ -Source $ToolName }

    if ($LASTEXITCODE -eq 0) {
        $result.Status = 'OK'
        Write-SuccessLog -Message 'Rust dependencies downloaded.' -Source $ToolName
    } else {
        $result.Status = 'Failed'
        Write-ErrorLog -Message "cargo fetch ended with error code: $LASTEXITCODE" -Source $ToolName
    }
} finally {
    Pop-Location
}

return $result
