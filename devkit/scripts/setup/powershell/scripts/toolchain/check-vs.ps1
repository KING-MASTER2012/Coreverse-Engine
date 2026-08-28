#Requires -Version 7.0
<#
.SYNOPSIS
    Ensures Visual Studio (MSVC toolset) is installed with the required
    workload/components, via vswhere.

.DESCRIPTION
    Unlike the other check-*.ps1 scripts, this one does not go through
    Invoke-ToolCheck: "sufficient" here means both a version check AND a
    component-completeness check (VC++ Tools, Windows SDK, ...), which the
    generic single-version hybrid flow doesn't model. Mirrors check-vcpkg.ps1's
    pattern of building the result object by hand instead.

    Strategy:
      1) vswhere present + latest instance satisfies RequiredVersion and has
         every id in RequiredComponents -> already satisfied, don't touch it.
      2) No VS instance at all -> winget install (falls back to the official
         vs_buildtools.exe bootstrapper if winget is unavailable/insufficient),
         requesting the required workload/components directly via --add.
      3) VS instance present but missing components / below RequiredVersion ->
         run the existing installer's setup.exe in `modify` mode to add the
         missing components in place.

    MSVC's own static analyzer (/analyze) and the Visual Studio Debugger ship
    as part of the VC++ Tools workload, so they don't need separate checks.
    MSVC has no native linter/formatter — clang-tidy/clang-format (see
    check-clang-tidy.ps1 / check-clang-format.ps1) cover that role instead.
#>

param(
    [string]$RequiredVersion = '17.0.0',
    [string[]]$RequiredComponents = @(
        'Microsoft.VisualStudio.Workload.VCTools',
        'Microsoft.VisualStudio.Component.VC.Tools.x86.x64',
        'Microsoft.VisualStudio.Component.Windows11SDK.26100'
    ),
    [switch]$DryRun
)

. "$PSScriptRoot/../common/logger.ps1"
. "$PSScriptRoot/../common/version-compare.ps1"
. "$PSScriptRoot/../package-managers/winget.ps1"

$ToolName = 'Visual Studio'

$result = [PSCustomObject]@{
    Tool            = $ToolName
    PreviousVersion = $null
    RequiredVersion = $RequiredVersion
    FinalVersion    = $null
    Source          = $null
    Status          = 'Unknown'
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsInstaller = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\setup.exe'

function Get-VsInstance {
    <# Returns $null if no instance satisfies every required component. #>
    if (-not (Test-Path $vswhere)) { return $null }

    $vswhereArgs = @('-latest', '-products', '*', '-format', 'json')
    foreach ($componentId in $RequiredComponents) {
        $vswhereArgs += @('-requires', $componentId)
    }

    $json = (& $vswhere @vswhereArgs) 2>$null
    if (-not $json) { return $null }

    try {
        $instances = $json | ConvertFrom-Json
        if ($instances -is [array]) { return $instances[0] } else { return $instances }
    } catch {
        return $null
    }
}

function Get-AnyVsInstance {
    <# Latest instance regardless of component completeness — used to decide
       install vs. modify. #>
    if (-not (Test-Path $vswhere)) { return $null }
    $json = (& $vswhere -latest -products * -format json) 2>$null
    if (-not $json) { return $null }
    try {
        $instances = $json | ConvertFrom-Json
        if ($instances -is [array]) { return $instances[0] } else { return $instances }
    } catch {
        return $null
    }
}

$instance = Get-VsInstance
if ($instance -and (Test-VersionAtLeast -CurrentRaw $instance.installationVersion -RequiredRaw $RequiredVersion)) {
    $result.PreviousVersion = $instance.installationVersion
    $result.FinalVersion = $instance.installationVersion
    $result.Source = 'AlreadySatisfied'
    $result.Status = 'OK'
    Write-SuccessLog -Message "Sufficient (>= $RequiredVersion) with all required components: $($instance.installationVersion)" -Source $ToolName
    return $result
}

$anyInstance = Get-AnyVsInstance
if ($anyInstance) {
    $result.PreviousVersion = $anyInstance.installationVersion
    Write-WarningLog -Message "Found $($anyInstance.installationVersion) but missing required components or below $RequiredVersion." -Source $ToolName
} else {
    Write-WarningLog -Message 'No Visual Studio installation found.' -Source $ToolName
}

if ($DryRun) {
    Write-InfoLog -Message '[DryRun] Installation/component completion was to be performed (winget -> upstream bootstrapper -> installer modify).' -Source $ToolName
    $result.Status = 'DryRun'
    return $result
}

$addArgs = @()
foreach ($componentId in $RequiredComponents) { $addArgs += @('--add', $componentId) }

try {
    if (-not $anyInstance) {
        # --- Fresh install ---
        $installed = $false
        if (Test-WingetAvailable) {
            Write-InfoLog -Message 'No existing installation; installing via winget (Visual Studio Build Tools)...' -Source $ToolName
            $overrideArgs = "--quiet --wait --norestart $($addArgs -join ' ')"
            & winget install --id 'Microsoft.VisualStudio.2022.BuildTools' --silent `
                --accept-package-agreements --accept-source-agreements `
                --override $overrideArgs 2>&1 | Out-Null
            $installed = $LASTEXITCODE -eq 0
        }

        if (-not $installed) {
            Write-WarningLog -Message 'winget unavailable/insufficient; falling back to the official vs_buildtools.exe bootstrapper.' -Source $ToolName
            $bootstrapperPath = Join-Path $env:TEMP 'vs_buildtools.exe'
            Invoke-WebRequest -Uri 'https://aka.ms/vs/17/release/vs_buildtools.exe' -OutFile $bootstrapperPath
            & $bootstrapperPath --quiet --wait --norestart @addArgs
            if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 3010) {
                throw "vs_buildtools.exe failed (exit code $LASTEXITCODE)."
            }
        }
        $result.Source = if ($installed) { 'winget' } else { 'Upstream' }
    } else {
        # --- Existing install, add missing components in place ---
        if (-not (Test-Path $vsInstaller)) {
            throw "Visual Studio Installer (setup.exe) not found next to vswhere.exe at '$vsInstaller'."
        }
        Write-InfoLog -Message "Adding missing components to existing install at '$($anyInstance.installationPath)'..." -Source $ToolName
        & $vsInstaller modify --installPath $anyInstance.installationPath --quiet --norestart --wait @addArgs
        if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 3010) {
            throw "Visual Studio Installer modify failed (exit code $LASTEXITCODE)."
        }
        $result.Source = 'Installer-Modify'
    }

    $instance = Get-VsInstance
    if ($instance -and (Test-VersionAtLeast -CurrentRaw $instance.installationVersion -RequiredRaw $RequiredVersion)) {
        $result.FinalVersion = $instance.installationVersion
        $result.Status = if ($result.PreviousVersion) { 'Upgraded' } else { 'Installed' }
        Write-SuccessLog -Message "Ready: $($instance.installationVersion) with all required components." -Source $ToolName
    } elseif ($anyInstance2 = Get-AnyVsInstance) {
        $result.FinalVersion = $anyInstance2.installationVersion
        $result.Status = 'Warning'
        Write-WarningLog -Message "Installed but still missing required components or below $RequiredVersion. Verify manually via the Visual Studio Installer." -Source $ToolName
    } else {
        $result.Status = 'Failed'
        Write-ErrorLog -Message 'No Visual Studio installation found after the install/modify step.' -Source $ToolName
    }
} catch {
    $result.Status = 'Failed'
    Write-ErrorLog -Message "Failed: $($_.Exception.Message)" -Source $ToolName
}

return $result
