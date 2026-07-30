#Requires -Version 7.0
<#
.SYNOPSIS
    Extracting and comparing versions from free text, such as the "--version" output of tools.
    Examples: "cargo 1.82.0 (8f40fc59f 2024-08-21)", "git version 2.45.1.windows.1",
    "go version go1.22.5 windows/amd64", "cmake version 3.30.2", "node v20.15.1"
#>

function ConvertTo-CleanVersion {
    param([Parameter(Mandatory)][AllowNull()][string]$RawVersionString)

    if (-not $RawVersionString) { return $null }

    if ($RawVersionString -match '(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)') {
        $verString = $Matches[1]
        $parts = $verString.Split('.')
        # The [version] type requires at least 2 parts; complete it to 3 parts (Major.Minor.Build)
        while ($parts.Count -lt 3) { $parts += '0' }
        # Remove the extra part (e.g., part 4 of git), [version] actually supports all 4 parts.
        try {
            return [version]($parts -join '.')
        } catch {
            return $null
        }
    }
    return $null
}

function Test-VersionAtLeast {
    <#
        $CurrentRaw and $RequiredRaw can be free text or plain string.
        If it cannot be parsed (tool not found / unexpected format), it returns $false - safe bet.
    #>
    param(
        [Parameter(Mandatory)][AllowNull()][string]$CurrentRaw,
        [Parameter(Mandatory)][string]$RequiredRaw
    )

    $current = ConvertTo-CleanVersion -RawVersionString $CurrentRaw
    $required = ConvertTo-CleanVersion -RawVersionString $RequiredRaw

    if (-not $current -or -not $required) { return $false }
    return ($current -ge $required)
}
