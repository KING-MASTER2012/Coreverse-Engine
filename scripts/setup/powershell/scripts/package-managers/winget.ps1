#Requires -Version 7.0
<#
.SYNOPSIS
    Hybrid release strategy step 1: installation/update via winget.
    # If insufficient, the calling check-*.ps1 script will fall back upstream.
#>

function Test-WingetAvailable {
    return [bool](Get-Command winget -ErrorAction SilentlyContinue)
}

function Sync-EnvironmentPath {
    <#
        Tools like winget/msiexec can update the system-wide PATH, but they don't reflect it in the current
        session. We combine the Machine and User PATHs and write them to the session PATH.
    #>
    $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $userPath    = [Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = @($machinePath, $userPath) -join ';'
}

function Install-WingetPackage {
    param(
        [Parameter(Mandatory)][string]$Id,
        [string]$Source = 'winget'
    )

    if (-not (Test-WingetAvailable)) {
        Write-WarningLog -Message 'winget not found on this system, this step skipped.' -Source $Source
        return $false
    }

    try {
        $installedList = & winget list --id $Id --accept-source-agreements 2>&1
        $alreadyInstalled = ($LASTEXITCODE -eq 0) -and ($installedList -match [regex]::Escape($Id))

        if ($alreadyInstalled) {
            Write-InfoLog -Message "winget upgrade --id $Id running..." -Source $Source
            & winget upgrade --id $Id --silent --accept-package-agreements --accept-source-agreements 2>&1 | Out-Null
        } else {
            Write-InfoLog -Message "winget install --id $Id running..." -Source $Source
            & winget install --id $Id --silent --accept-package-agreements --accept-source-agreements 2>&1 | Out-Null
        }

        Sync-EnvironmentPath
        return $true
    } catch {
        Write-WarningLog -Message "winget process unsuccessful: $($_.Exception.Message)" -Source $Source
        return $false
    }
}
