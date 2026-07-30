#Requires -Version 7.0
param(
    [string[]]$Projects = @('launcher'),
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"

$ToolName = 'npm Deps'
$results = @()

$npmCmd = Get-Command npm -ErrorAction SilentlyContinue

foreach ($proj in $Projects) {
    $projResult = [PSCustomObject]@{ Tool = "npm Deps ($proj)"; Status = 'Unknown'; Detail = $null }

    $packageJsonPath = Join-Path $proj 'package.json'
    if (-not (Test-Path $packageJsonPath)) {
        Write-WarningLog -Message "package.json not found ($packageJsonPath), skipped." -Source $ToolName
        $projResult.Status = 'Skipped'
        $results += $projResult
        continue
    }

    if ($DryRun) {
        Write-InfoLog -Message "[DryRun] In '$proj', 'npm install' was to be run." -Source $ToolName
        $projResult.Status = 'DryRun'
        $results += $projResult
        continue
    }

    if (-not $npmCmd) {
        Write-ErrorLog -Message 'npm not found on PATH. First, the toolchain phase must be completed' -Source $ToolName
        $projResult.Status = 'Failed'
        $results += $projResult
        continue
    }

    Push-Location $proj
    try {
        $lockExists = Test-Path 'package-lock.json'
        $npmArgs = if ($lockExists) { @('ci') } else { @('install') }

        Write-InfoLog -Message "In '$proj', 'npm $($npmArgs -join ' ')' running..." -Source $ToolName
        & npm @npmArgs 2>&1 | ForEach-Object { Write-PlainLog -Message $_ -Source $ToolName }

        if ($LASTEXITCODE -eq 0) {
            $projResult.Status = 'OK'
            Write-SuccessLog -Message "'$proj' dependencies downloaded." -Source $ToolName
        } else {
            $projResult.Status = 'Failed'
            Write-ErrorLog -Message "'npm $($npmArgs -join ' ')' ended with error code: $LASTEXITCODE" -Source $ToolName
        }
    } finally {
        Pop-Location
    }

    $results += $projResult
}

return $results
