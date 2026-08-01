#Requires -Version 7.0
<#
.SYNOPSIS
    Coreverse Bootstrap - log functions with common colors.
    All scripts use this file by dot source.
#>

$Script:CVLogColors = @{
    Info    = 'Cyan'
    Success = 'Green'
    Warning = 'Yellow'
    Error   = 'Red'
}

$Script:CVLogPrefixes = @{
    Info    = '[INFO]'
    Success = '[OK]'
    Warning = '[WARN]'
    Error   = '[ERR]'
}

$Script:CVLogEmojis = @{
    Info    = [char]::ConvertFromUtf32(0x1F535)   # 🔵
    Success = [char]::ConvertFromUtf32(0x1F7E2)   # 🟢
    Warning = [char]::ConvertFromUtf32(0x1F7E1)   # 🟡
    Error   = [char]::ConvertFromUtf32(0x1F534)   # 🔴
}

function Write-BootstrapLog {
    <#
        The common starting point for all log levels. -The `Source` parameter is used
        to distinguish which tool is generating the logs in parallel tasks.
    #>
    param(
        [Parameter(Mandatory)][string]$Message,
        [ValidateSet('Info', 'Success', 'Warning', 'Error', 'Plain')]
        [string]$Level = 'Plain',
        [string]$Source
    )

    $sourceTag = if ($Source) { "[$Source] " } else { '' }

    if ($Level -eq 'Plain') {
        Write-Host "$sourceTag$Message"
        return
    }

    $emoji = $Script:CVLogEmojis[$Level]
    $prefix = $Script:CVLogPrefixes[$Level]
    $line = "$emoji $sourceTag$prefix $Message"
    Write-Host $line -ForegroundColor $Script:CVLogColors[$Level]
}

function Write-InfoLog    { param([Parameter(Mandatory)][string]$Message, [string]$Source) Write-BootstrapLog -Message $Message -Level 'Info'    -Source $Source }
function Write-SuccessLog { param([Parameter(Mandatory)][string]$Message, [string]$Source) Write-BootstrapLog -Message $Message -Level 'Success' -Source $Source }
function Write-WarningLog { param([Parameter(Mandatory)][string]$Message, [string]$Source) Write-BootstrapLog -Message $Message -Level 'Warning' -Source $Source }
function Write-ErrorLog   { param([Parameter(Mandatory)][string]$Message, [string]$Source) Write-BootstrapLog -Message $Message -Level 'Error'   -Source $Source }
function Write-PlainLog   { param([Parameter(Mandatory)][string]$Message, [string]$Source) Write-BootstrapLog -Message $Message -Level 'Plain'   -Source $Source }

function Write-Banner {
    param([Parameter(Mandatory)][string]$Title)
    $line = '=' * ($Title.Length + 4)
    Write-Host ''
    Write-Host $line -ForegroundColor DarkCyan
    Write-Host "  $Title" -ForegroundColor DarkCyan
    Write-Host $line -ForegroundColor DarkCyan
}
