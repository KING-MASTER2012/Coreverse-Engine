#Requires -Version 7.0
param(
    [string[]]$Modules = @('server'),
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"

$ToolName = 'Go Deps'
$results = @()

$goCmd = Get-Command go -ErrorAction SilentlyContinue

foreach ($mod in $Modules) {
    $modResult = [PSCustomObject]@{ Tool = "Go Deps ($mod)"; Status = 'Unknown'; Detail = $null }

    $goModPath = Join-Path $mod 'go.mod'
    if (-not (Test-Path $goModPath)) {
        Write-WarningLog -Message "go.mod not found ($goModPath), skipped." -Source $ToolName
        $modResult.Status = 'Skipped'
        $results += $modResult
        continue
    }

    if ($DryRun) {
        Write-InfoLog -Message "[DryRun] In '$mod', 'go mod download' was to be run." -Source $ToolName
        $modResult.Status = 'DryRun'
        $results += $modResult
        continue
    }

    if (-not $goCmd) {
        Write-ErrorLog -Message 'go not found on PATH. First, the toolchain phase must be completed..' -Source $ToolName
        $modResult.Status = 'Failed'
        $results += $modResult
        continue
    }

    Push-Location $mod
    try {
        Write-InfoLog -Message "In '$mod', go mod download running..." -Source $ToolName
        & go mod download 2>&1 | ForEach-Object { Write-PlainLog -Message $_ -Source $ToolName }

        if ($LASTEXITCODE -eq 0) {
            $modResult.Status = 'OK'
            Write-SuccessLog -Message "'$mod' dependencies downloaded." -Source $ToolName
        } else {
            $modResult.Status = 'Failed'
            Write-ErrorLog -Message "go mod download ended with error code: $LASTEXITCODE" -Source $ToolName
        }
    } finally {
        Pop-Location
    }

    $results += $modResult
}

return $results
