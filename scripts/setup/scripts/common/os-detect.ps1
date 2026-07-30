#Requires -Version 7.0
<#
.SYNOPSIS
    bootstrap.ps1 only works on Windows; this module collects environment/architecture/authorization
    information. Linux distribution detection will be handled separately in bootstrap.sh.
#>

function Test-IsAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-OSInfo {
    $os = Get-CimInstance -ClassName Win32_OperatingSystem
    $arch = if (-not [Environment]::Is64BitOperatingSystem) {
        'x86'
    } elseif ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
        'ARM64'
    } else {
        'x64'
    }

    [PSCustomObject]@{
        Platform     = 'Windows'
        Caption      = $os.Caption
        Version      = $os.Version
        BuildNumber  = $os.BuildNumber
        Architecture = $arch
        IsAdmin      = Test-IsAdministrator
        PSVersion    = $PSVersionTable.PSVersion.ToString()
        PSEdition    = $PSVersionTable.PSEdition
    }
}

function Test-MinimumPSVersion {
    <#
        The `ForEach-Object -Parallel` command used within `Invoke-TaskGraph` requires PowerShell 7.0+.
        If run on Windows PowerShell 5.1, a clear warning will be issued and the system will exit.
    #>
    param([int]$MinimumMajor = 7)
    return $PSVersionTable.PSVersion.Major -ge $MinimumMajor
}
