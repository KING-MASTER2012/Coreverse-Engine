#Requires -Version 7.0
<#
.SYNOPSIS
    A kernel module that runs independent tasks in parallel and dependent tasks sequentially/layered.

    Each task object expects a structure like this:
    @{
      Name = 'Git' # unique name
      ScriptPath = 'C:\...\check-git.ps1'
      Arguments = @{ RequiredVersion = '2.40.0'; DryRun = $false }
      DependsOn = @() # dependency names like @('Rustup')
     }

     ForEach-Object -Parallel runs each element in a separate runspace; therefore, each
     ScriptPath must be self-sufficient
     (it dot-sources its dependencies like logger.ps1 / version-compare.ps1 / winget.ps1
     internally via $PSScriptRoot).
#>

function Invoke-TaskGraph {
    param(
        [Parameter(Mandatory)][array]$Tasks,
        [int]$ThrottleLimit = [Environment]::ProcessorCount
    )

    $completed = @{}
    $results   = @()
    $remaining = @($Tasks)

    while ($remaining.Count -gt 0) {
        $runnable = @($remaining | Where-Object {
            $deps = $_.DependsOn
            if (-not $deps -or $deps.Count -eq 0) { return $true }
            $unmet = @($deps | Where-Object { -not $completed.ContainsKey($_) })
            return ($unmet.Count -eq 0)
        })

        if ($runnable.Count -eq 0) {
            $stuckNames = ($remaining | ForEach-Object { $_.Name }) -join ', '
            throw "An unsolvable or cyclical addiction was detected: $stuckNames"
        }

        $layerResults = $runnable | ForEach-Object -Parallel {
            $task = $_
            $params = if ($task.Arguments) { $task.Arguments } else { @{} }
            try {
                $output = & $task.ScriptPath @params
                [PSCustomObject]@{
                    Name    = $task.Name
                    Success = $true
                    Result  = $output
                    Error   = $null
                }
            } catch {
                [PSCustomObject]@{
                    Name    = $task.Name
                    Success = $false
                    Result  = $null
                    Error   = $_.Exception.Message
                }
            }
        } -ThrottleLimit $ThrottleLimit

        foreach ($r in $layerResults) { $completed[$r.Name] = $true }
        $results += $layerResults

        $runnableNames = @($runnable | ForEach-Object { $_.Name })
        $remaining = @($remaining | Where-Object { $_.Name -notin $runnableNames })
    }

    return $results
}

function Invoke-ParallelTasks {
    <#
        A simple parallel runner for completely independent tasks with no dependencies.
        # (e.g., cargo/npm/go/vcpkg installations don't wait for each other during the dependency-parse phase)
    #>
    param(
        [Parameter(Mandatory)][array]$Tasks,
        [int]$ThrottleLimit = [Environment]::ProcessorCount
    )

    return $Tasks | ForEach-Object -Parallel {
        $task = $_
        $params = if ($task.Arguments) { $task.Arguments } else { @{} }
        try {
            $output = & $task.ScriptPath @params
            [PSCustomObject]@{ Name = $task.Name; Success = $true; Result = $output; Error = $null }
        } catch {
            [PSCustomObject]@{ Name = $task.Name; Success = $false; Result = $null; Error = $_.Exception.Message }
        }
    } -ThrottleLimit $ThrottleLimit
}
