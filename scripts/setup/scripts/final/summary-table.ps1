#Requires -Version 7.0
<#
.SYNOPSIS
    The toolchain, dependency, and cmake results are printed as a colorful, aligned table.
    Format-Table doesn't allow cell-based coloring, so it's written manually.
#>

. "$PSScriptRoot/../common/logger.ps1"

$Script:CVStatusColors = @{
    OK           = 'Green'
    Installed    = 'Green'
    Upgraded     = 'Green'
    Warning      = 'Yellow'
    DryRun       = 'Cyan'
    Skipped      = 'DarkGray'
    Failed       = 'Red'
    Unknown      = 'DarkGray'
}

function Show-SummaryTable {
    param([Parameter(Mandatory)][array]$Results)

    Write-Banner -Title 'Coreverse Bootstrap - Summary'

    $nameWidth    = [Math]::Max(20, ($Results | ForEach-Object { $_.Tool.Length } | Measure-Object -Maximum).Maximum + 2)
    $versionWidth = [Math]::Max(15, ($Results | ForEach-Object { ("$($_.FinalVersion)").Length } | Measure-Object -Maximum).Maximum + 2)
    $statusWidth  = 12

    $header = "{0,-$nameWidth}{1,-$versionWidth}{2,-$statusWidth}" -f 'TOOL/STEP', 'VERSION', 'STATUS'
    Write-Host $header -ForegroundColor White
    Write-Host ('-' * ($nameWidth + $versionWidth + $statusWidth)) -ForegroundColor DarkGray

    $failedCount = 0
    $warningCount = 0

    foreach ($r in $Results) {
        $version = if ($r.FinalVersion) { "$($r.FinalVersion)" } else { '-' }
        # Uzun ham "--version" ciktilarini tabloda kisalt
        if ($version.Length -gt ($versionWidth - 2)) {
            $version = $version.Substring(0, $versionWidth - 5) + '...'
        }

        $status = if ($r.Status) { $r.Status } else { 'Unknown' }
        $color = $Script:CVStatusColors[$status]
        if (-not $color) { $color = 'White' }

        if ($status -eq 'Failed') { $failedCount++ }
        if ($status -eq 'Warning') { $warningCount++ }

        $row = "{0,-$nameWidth}{1,-$versionWidth}" -f $r.Tool, $version
        Write-Host $row -NoNewline
        Write-Host $status -ForegroundColor $color
    }

    Write-Host ('-' * ($nameWidth + $versionWidth + $statusWidth)) -ForegroundColor DarkGray
    Write-Host ''

    if ($failedCount -gt 0) {
        Write-ErrorLog -Message "Bootstrap completed with $failedCount error"
    } elseif ($warningCount -gt 0) {
        Write-WarningLog -Message "Bootstrap completed with $warningCount warning"
    } else {
        Write-SuccessLog -Message 'Bootstrap completed successfully'
    }

    return [PSCustomObject]@{
        FailedCount  = $failedCount
        WarningCount = $warningCount
    }
}
