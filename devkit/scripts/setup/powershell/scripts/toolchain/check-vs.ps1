#Requires -Version 7.0
<#
.SYNOPSIS
    Ensures the newest installed Visual Studio instance has the required
    MSVC and Windows SDK components.

.DESCRIPTION
    Visual Studio discovery is fully dynamic and machine-independent.

    Supported machine states:
      - No Visual Studio installation
      - One Visual Studio installation
      - Multiple Visual Studio installations

    ONLY the newest Visual Studio instance is considered.

    Older Visual Studio installations are never:
      - checked
      - modified
      - upgraded
      - used as a fallback

    Required components:
      - Microsoft.VisualStudio.Component.VC.Tools.x86.x64
      - Microsoft.VisualStudio.Component.Windows11SDK.26100

    Flow:
      1. Dynamically locate vswhere.exe.
      2. Select the newest Visual Studio instance.
      3. Validate its version and required components.
      4. If required, dynamically locate setup.exe.
      5. Modify ONLY the newest Visual Studio instance.
      6. Wait for setup.exe using PowerShell's -Wait.
      7. Re-discover the newest instance.
      8. Validate again.

    If no Visual Studio installation exists:
      - Try winget.
      - Fall back to the official Visual Studio 2026 Build Tools bootstrapper.

.NOTES
    Visual Studio Installer uses the machine/installer locale for its own
    output. The Coreverse bootstrap log itself remains in English.
#>

param(
    [string]$RequiredVersion = '17.0.0',

    [string[]]$RequiredComponents = @(
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


# ============================================================================
# Visual Studio discovery
# ============================================================================

function Find-VsWhere {

    # ------------------------------------------------------------------------
    # 1. PATH
    # ------------------------------------------------------------------------

    $command = Get-Command 'vswhere.exe' -ErrorAction SilentlyContinue

    if ($command -and $command.Source) {

        if (Test-Path $command.Source) {
            return (Resolve-Path $command.Source).Path
        }
    }


    # ------------------------------------------------------------------------
    # 2. Standard Visual Studio Installer locations
    # ------------------------------------------------------------------------

    $candidates = @()

    if ($env:ProgramFiles) {

        $candidates += Join-Path `
            $env:ProgramFiles `
            'Microsoft Visual Studio\Installer\vswhere.exe'
    }

    if (${env:ProgramFiles(x86)}) {

        $candidates += Join-Path `
            ${env:ProgramFiles(x86)} `
            'Microsoft Visual Studio\Installer\vswhere.exe'
    }


    foreach ($candidate in ($candidates | Select-Object -Unique)) {

        if (Test-Path $candidate) {
            return (Resolve-Path $candidate).Path
        }
    }


    # ------------------------------------------------------------------------
    # 3. Recursive fallback
    # ------------------------------------------------------------------------

    $roots = @(
        $env:ProgramFiles,
        ${env:ProgramFiles(x86)}
    ) |
        Where-Object {
            $_ -and (Test-Path $_)
        } |
        Select-Object -Unique


    foreach ($root in $roots) {

        $visualStudioRoot = Join-Path `
            $root `
            'Microsoft Visual Studio'

        if (-not (Test-Path $visualStudioRoot)) {
            continue
        }

        try {

            $found = Get-ChildItem `
                -Path $visualStudioRoot `
                -Filter 'vswhere.exe' `
                -File `
                -Recurse `
                -ErrorAction SilentlyContinue |
                Select-Object -First 1

            if ($found) {
                return $found.FullName
            }

        }
        catch {
            # Continue with the next discovery strategy.
        }
    }

    return $null
}


function Find-VsInstaller {

    param(
        [Parameter(Mandatory)]
        [string]$VsWherePath
    )

    $installerDirectory = Split-Path `
        -Parent `
        $VsWherePath

    $setupPath = Join-Path `
        $installerDirectory `
        'setup.exe'

    if (Test-Path $setupPath) {
        return (Resolve-Path $setupPath).Path
    }

    return $null
}


# ============================================================================
# Newest Visual Studio instance
# ============================================================================

function Get-NewestVsInstance {

    param(
        [Parameter(Mandatory)]
        [string]$VsWherePath
    )

    $json = & $VsWherePath `
        -latest `
        -products '*' `
        -format json `
        -utf8 `
        2>$null

    $exitCode = $LASTEXITCODE

    if ($exitCode -ne 0 -or -not $json) {
        return $null
    }

    try {

        $instances = $json | ConvertFrom-Json

        if ($instances -is [array]) {

            if ($instances.Count -eq 0) {
                return $null
            }

            return $instances[0]
        }

        return $instances
    }
    catch {

        return $null
    }
}

# ============================================================================
# Component validation
# ============================================================================

function Test-VsComponent {

    param(
        [Parameter(Mandatory)]
        [string]$VsWherePath,

        [Parameter(Mandatory)]
        $Instance,

        [Parameter(Mandatory)]
        [string]$ComponentId
    )

    # Ask vswhere for the newest installed instance that contains this
    # component, then compare its installation path with the already-selected
    # newest Visual Studio instance.
    #
    # We intentionally do NOT use -path together with -requires because
    # vswhere does not support -path with other selection options.

    $matchedPath = & $VsWherePath `
        -latest `
        -products '*' `
        -requires $ComponentId `
        -property installationPath `
        2>$null

    $exitCode = $LASTEXITCODE

    if ($exitCode -ne 0 -or -not $matchedPath) {
        return $false
    }

    $matchedPath = ($matchedPath | Select-Object -First 1).ToString().Trim()

    if ([string]::IsNullOrWhiteSpace($matchedPath)) {
        return $false
    }

    try {
        $selectedPath = [System.IO.Path]::GetFullPath(
            $Instance.installationPath
        )

        $componentPath = [System.IO.Path]::GetFullPath(
            $matchedPath
        )

        return (
            [System.StringComparer]::OrdinalIgnoreCase.Equals(
                $selectedPath,
                $componentPath
            )
        )
    }
    catch {
        return $false
    }
}


function Get-MissingVsComponents {

    param(
        [Parameter(Mandatory)]
        [string]$VsWherePath,

        [Parameter(Mandatory)]
        $Instance
    )

    $missing = @()

    foreach ($componentId in $RequiredComponents) {

        $present = Test-VsComponent `
            -VsWherePath $VsWherePath `
            -Instance $Instance `
            -ComponentId $componentId

        if (-not $present) {
            $missing += $componentId
        }
    }

    return @($missing)
}


function Test-VsRequirements {

    param(
        [Parameter(Mandatory)]
        [string]$VsWherePath,

        [Parameter(Mandatory)]
        $Instance
    )

    $versionSufficient = Test-VersionAtLeast `
        -CurrentRaw $Instance.installationVersion `
        -RequiredRaw $RequiredVersion

    $missingComponents = Get-MissingVsComponents `
        -VsWherePath $VsWherePath `
        -Instance $Instance

    $componentsSufficient = ($missingComponents.Count -eq 0)

    return [PSCustomObject]@{
        VersionSufficient     = $versionSufficient
        ComponentsSufficient = $componentsSufficient
        MissingComponents     = $missingComponents
        Satisfied             = (
            $versionSufficient -and
            $componentsSufficient
        )
    }
}

# ============================================================================
# Discover Visual Studio Locator
# ============================================================================

$vswhere = Find-VsWhere

if ($vswhere) {

    Write-InfoLog `
        -Message "Using Visual Studio Locator: '$vswhere'." `
        -Source $ToolName
}
else {

    Write-WarningLog `
        -Message 'vswhere.exe could not be located.' `
        -Source $ToolName
}


# ============================================================================
# Select ONLY the newest Visual Studio instance
# ============================================================================

$instance = $null

if ($vswhere) {

    $instance = Get-NewestVsInstance `
        -VsWherePath $vswhere
}


# ============================================================================
# Existing Visual Studio
# ============================================================================

if ($instance) {

    $result.PreviousVersion = $instance.installationVersion

    Write-InfoLog `
        -Message "Newest Visual Studio found: $($instance.installationVersion) at '$($instance.installationPath)'." `
        -Source $ToolName


    $validation = Test-VsRequirements `
        -VsWherePath $vswhere `
        -Instance $instance


    if ($validation.Satisfied) {

        $result.FinalVersion = $instance.installationVersion
        $result.Source = 'AlreadySatisfied'
        $result.Status = 'OK'

        Write-SuccessLog `
            -Message "Sufficient (>= $RequiredVersion) with all required components: $($instance.installationVersion)" `
            -Source $ToolName

        return $result
    }


    if (-not $validation.VersionSufficient) {

        Write-WarningLog `
            -Message "Newest Visual Studio $($instance.installationVersion) is below required version $RequiredVersion." `
            -Source $ToolName
    }


    if (-not $validation.ComponentsSufficient) {

        Write-WarningLog `
            -Message "Newest Visual Studio is missing required components: $($validation.MissingComponents -join ', ')." `
            -Source $ToolName
    }
}
else {

    Write-WarningLog `
        -Message 'No Visual Studio installation was found.' `
        -Source $ToolName
}


# ============================================================================
# Dry run
# ============================================================================

if ($DryRun) {

    Write-InfoLog `
        -Message '[DryRun] No Visual Studio installation was modified.' `
        -Source $ToolName

    $result.Status = 'DryRun'

    return $result
}


try {

    # =========================================================================
    # Installer arguments
    # =========================================================================

    $addArgs = @()

    foreach ($componentId in $RequiredComponents) {

        $addArgs += @(
            '--add',
            $componentId
        )
    }


    # =========================================================================
    # No Visual Studio -> fresh installation
    # =========================================================================

    if (-not $instance) {

        Write-InfoLog `
            -Message 'No Visual Studio installation found; installing Visual Studio 2026 Build Tools...' `
            -Source $ToolName


        $installed = $false


        # ---------------------------------------------------------------------
        # winget
        # ---------------------------------------------------------------------

        if (Test-WingetAvailable) {

            Write-InfoLog `
                -Message 'Attempting Visual Studio 2026 Build Tools installation via winget...' `
                -Source $ToolName


            $overrideArgs = `
                "--quiet --wait --norestart $($addArgs -join ' ')"


            & winget install `
                --id 'Microsoft.VisualStudio.2026.BuildTools' `
                --silent `
                --accept-package-agreements `
                --accept-source-agreements `
                --override $overrideArgs `
                2>&1 | Out-Null


            $wingetExitCode = $LASTEXITCODE


            if (
                $wingetExitCode -eq 0 -or
                $wingetExitCode -eq 3010
            ) {

                $installed = $true
                $result.Source = 'winget'
            }
        }


        # ---------------------------------------------------------------------
        # Official bootstrapper
        # ---------------------------------------------------------------------

        if (-not $installed) {

            Write-InfoLog `
                -Message 'Using the official Visual Studio 2026 Build Tools bootstrapper...' `
                -Source $ToolName


            $bootstrapperPath = Join-Path `
                $env:TEMP `
                'vs_buildtools_2026.exe'


            Invoke-WebRequest `
                -Uri 'https://aka.ms/vs/18/release/vs_buildtools.exe' `
                -OutFile $bootstrapperPath


            # --wait belongs to the bootstrapper.
            $bootstrapperArguments = @(
                '--quiet',
                '--wait',
                '--norestart'
            ) + $addArgs


            $process = Start-Process `
                -FilePath $bootstrapperPath `
                -ArgumentList $bootstrapperArguments `
                -WorkingDirectory $env:TEMP `
                -Wait `
                -PassThru


            $bootstrapperExitCode = $process.ExitCode


            if (
                $bootstrapperExitCode -ne 0 -and
                $bootstrapperExitCode -ne 3010
            ) {

                throw `
                    "Visual Studio 2026 Build Tools bootstrapper failed (exit code $bootstrapperExitCode)."
            }


            $result.Source = 'Upstream'
        }


        # ---------------------------------------------------------------------
        # Re-discover after installation
        # ---------------------------------------------------------------------

        $vswhere = Find-VsWhere


        if (-not $vswhere) {

            throw `
                'Visual Studio installation completed, but vswhere.exe could not be located.'
        }


        $instance = Get-NewestVsInstance `
            -VsWherePath $vswhere


        if (-not $instance) {

            throw `
                'Visual Studio installation completed, but the installed instance could not be discovered.'
        }
    }


    # ============================================================================
    # Existing installation -> modify ONLY newest instance
    # ============================================================================

    else {

        $vsInstaller = Find-VsInstaller `
            -VsWherePath $vswhere

        if (-not $vsInstaller) {
            throw `
                "Visual Studio Installer setup.exe could not be located relative to '$vswhere'."
        }

        Write-InfoLog `
            -Message "Modifying ONLY the newest Visual Studio instance at '$($instance.installationPath)'..." `
            -Source $ToolName

        Write-InfoLog `
            -Message "Visual Studio Installer: '$vsInstaller'." `
            -Source $ToolName

        # IMPORTANT:
        # Use ProcessStartInfo.ArgumentList so paths containing spaces remain
        # a single argument. Start-Process -ArgumentList can otherwise produce
        # an ambiguous command line when arguments are not explicitly quoted.
        $startInfo = [System.Diagnostics.ProcessStartInfo]::new()

        $startInfo.FileName = $vsInstaller
        $startInfo.WorkingDirectory = $env:TEMP
        $startInfo.UseShellExecute = $false
        $startInfo.CreateNoWindow = $false

        [void]$startInfo.ArgumentList.Add('modify')
        [void]$startInfo.ArgumentList.Add('--installPath')
        [void]$startInfo.ArgumentList.Add($instance.installationPath)
        [void]$startInfo.ArgumentList.Add('--passive')
        [void]$startInfo.ArgumentList.Add('--norestart')

        foreach ($componentId in $RequiredComponents) {
            [void]$startInfo.ArgumentList.Add('--add')
            [void]$startInfo.ArgumentList.Add($componentId)
        }

        $process = [System.Diagnostics.Process]::new()
        $process.StartInfo = $startInfo

        if (-not $process.Start()) {
            throw "Failed to start Visual Studio Installer."
        }

        $process.WaitForExit()

        $modifyExitCode = $process.ExitCode

        $process.Dispose()

        if (
            $modifyExitCode -ne 0 -and
            $modifyExitCode -ne 3010
        ) {
            throw "Visual Studio Installer modify failed (exit code $modifyExitCode)."
        }

        $result.Source = 'Installer-Modify'
    }


    # =========================================================================
    # Final verification
    # =========================================================================

    $vswhere = Find-VsWhere


    if (-not $vswhere) {

        throw `
            'vswhere.exe could not be located during final verification.'
    }


    $instance = Get-NewestVsInstance `
        -VsWherePath $vswhere


    if (-not $instance) {

        throw `
            'No Visual Studio installation could be discovered after the install/modify operation.'
    }


    $result.FinalVersion = $instance.installationVersion


    $validation = Test-VsRequirements `
        -VsWherePath $vswhere `
        -Instance $instance


    if ($validation.Satisfied) {

        if ($result.PreviousVersion) {
            $result.Status = 'Upgraded'
        }
        else {
            $result.Status = 'Installed'
        }


        Write-SuccessLog `
            -Message "Ready: newest Visual Studio $($instance.installationVersion) has all required components." `
            -Source $ToolName
    }
    else {

        $result.Status = 'Warning'


        if (-not $validation.VersionSufficient) {

            Write-WarningLog `
                -Message "Newest Visual Studio $($instance.installationVersion) is below required version $RequiredVersion." `
                -Source $ToolName
        }


        if (-not $validation.ComponentsSufficient) {

            Write-WarningLog `
                -Message "Newest Visual Studio $($instance.installationVersion) is still missing required components: $($validation.MissingComponents -join ', ')." `
                -Source $ToolName
        }
    }

}
catch {

    $result.Status = 'Failed'

    $exceptionMessage = $_.Exception.Message


    if ([string]::IsNullOrWhiteSpace($exceptionMessage)) {

        $exceptionMessage = `
            'Unknown error while checking or modifying Visual Studio.'
    }


    Write-ErrorLog `
        -Message "Failed: $exceptionMessage" `
        -Source $ToolName
}

return $result
