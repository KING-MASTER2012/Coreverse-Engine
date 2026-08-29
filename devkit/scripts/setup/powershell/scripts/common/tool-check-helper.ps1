#Requires -Version 7.0
<#
.SYNOPSIS
    The common check/installation flow called by every check-*.ps1 script.

    Strategy (hybrid, user-selected option):
      1) Is the tool in the PATH and is the version sufficient? -> If sufficient, do not touch.
      2) If insufficient/unavailable: Install/update with winget. -> Finish if sufficient.
      3) If that is still not enough: Use upstream (official source). -> If the result is still insufficient, continue with WARNING
      (the process is not stopped), FAILED if completely unsuccessful.
#>

. "$PSScriptRoot/logger.ps1"
. "$PSScriptRoot/version-compare.ps1"
. "$PSScriptRoot/tool-lock.ps1"
. "$PSScriptRoot/../package-managers/winget.ps1"

function Invoke-ToolCheck {
    param(
        [Parameter(Mandatory)][string]$ToolName,
        [Parameter(Mandatory)][string]$RequiredVersion,
        [Parameter(Mandatory)][scriptblock]$GetVersionRaw,
        [string]$WingetId,
        [Parameter(Mandatory)][scriptblock]$UpstreamInstall,
        [switch]$DryRun,
        # Optional: name of a cross-runspace mutex (see tool-lock.ps1) to hold
        # while running UpstreamInstall. Use this when the upstream installer
        # is a CLI that can't handle being invoked concurrently with itself
        # (e.g. 'cargo-install' for `cargo install`, 'rustup-component' for
        # `rustup component add`) - NOT needed for tools that already go
        # through winget, since winget serializes its own operations.
        [string]$ToolLockName
    )

    $result = [PSCustomObject]@{
        Tool            = $ToolName
        PreviousVersion = $null
        RequiredVersion = $RequiredVersion
        FinalVersion    = $null
        Source          = $null
        Status          = 'Unknown'
    }

    $raw = & $GetVersionRaw
    $result.PreviousVersion = $raw

    if ($raw) {
        Write-InfoLog -Message "Found: $raw" -Source $ToolName
    } else {
        Write-WarningLog -Message 'Not found on PATH.' -Source $ToolName
    }

    if ($raw -and (Test-VersionAtLeast -CurrentRaw $raw -RequiredRaw $RequiredVersion)) {
        $result.FinalVersion = $raw
        $result.Source = 'AlreadySatisfied'
        $result.Status = 'OK'
        Write-SuccessLog -Message "The version is sufficient (>= $RequiredVersion)." -Source $ToolName
        return $result
    }

    if ($DryRun) {
        Write-InfoLog -Message '[DryRun] Installation/update was to be performed (winget -> upstream).' -Source $ToolName
        $result.Status = 'DryRun'
        return $result
    }

    # --- Step 1: winget ---
    if ($WingetId -and (Test-WingetAvailable)) {
        Install-WingetPackage -Id $WingetId -Source $ToolName | Out-Null
        $raw = & $GetVersionRaw
        if ($raw -and (Test-VersionAtLeast -CurrentRaw $raw -RequiredRaw $RequiredVersion)) {
            $result.FinalVersion = $raw
            $result.Source = 'winget'
            $result.Status = if ($result.PreviousVersion) { 'Upgraded' } else { 'Installed' }
            Write-SuccessLog -Message "Installed/upgraded (winget): $raw" -Source $ToolName
            return $result
        }
    }

    # --- Step 2: upstream fallback ---
    Write-WarningLog -Message 'winget proved insufficient; the information is being transferred to the official upstream source.' -Source $ToolName

    $toolLock = $null
    if ($ToolLockName) {
        $toolLock = Enter-ToolLock -LockName $ToolLockName -Source $ToolName
        if (-not $toolLock) {
            $result.Status = 'Failed'
            return $result
        }
    }

    try {
        & $UpstreamInstall
        $raw = & $GetVersionRaw

        if ($raw -and (Test-VersionAtLeast -CurrentRaw $raw -RequiredRaw $RequiredVersion)) {
            $result.FinalVersion = $raw
            $result.Source = 'Upstream'
            $result.Status = if ($result.PreviousVersion) { 'Upgraded' } else { 'Installed' }
            Write-SuccessLog -Message "Installed/updated (upstream): $raw" -Source $ToolName
        } elseif ($raw) {
            $result.FinalVersion = $raw
            $result.Source = 'Upstream'
            $result.Status = 'Warning'
            Write-WarningLog -Message "Installed but current version is lower than wanted version: $raw (the wanted >= $RequiredVersion). Continuing" -Source $ToolName
        } else {
            $result.Status = 'Failed'
            Write-ErrorLog -Message 'After installation, the vehicle could not be found on the PATH.' -Source $ToolName
        }
    } catch {
        $result.Status = 'Failed'
        Write-ErrorLog -Message "Installation unsuccessful: $($_.Exception.Message)" -Source $ToolName
    } finally {
        if ($toolLock) {
            Exit-ToolLock -Mutex $toolLock
        }
    }

    return $result
}
