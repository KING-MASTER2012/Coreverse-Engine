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
        [Parameter(Mandatory)]
        [array]$Tasks,

        [int]$ThrottleLimit = [Environment]::ProcessorCount
    )

    $completed = @{}
    $results   = @()
    $remaining = @($Tasks)

    while ($remaining.Count -gt 0) {

        $runnable = @(
        $remaining | Where-Object {

            if (-not $_.DependsOn -or $_.DependsOn.Count -eq 0) {
                return $true
            }

            foreach ($dep in $_.DependsOn) {
                if (-not $completed.ContainsKey($dep)) {
                    return $false
                }
            }

            return $true
        }
        )

        if ($runnable.Count -eq 0) {
            throw "Circular dependency detected."
        }

        $jobs = foreach ($task in $runnable) {

            Start-ThreadJob -Name $task.Name -ArgumentList $task -ScriptBlock {

                param($task)

                try {

                    $params = if ($task.Arguments) {
                        $task.Arguments
                    }
                    else {
                        @{}
                    }

                    $result = & $task.ScriptPath @params

                    if ($null -eq $result) {
                        throw "Script returned no object."
                    }

                    [PSCustomObject]@{
                        Name    = $task.Name
                        Success = $true
                        Result  = $result
                        Error   = $null
                    }

                }
                catch {

                    [PSCustomObject]@{
                        Name    = $task.Name
                        Success = $false
                        Result  = $null
                        Error   = $_ | Out-String
                    }

                }

            }

        }

        Wait-Job $jobs | Out-Null

        $layer = foreach ($job in $jobs) {

            $r = Receive-Job $job

            Remove-Job $job

            $r

        }

        foreach ($r in $layer) {

            if ($r.Success) {
                $completed[$r.Name] = $true
            }

        }

        $results += $layer

        $remaining = @(
        $remaining | Where-Object {
            $_.Name -notin $runnable.Name
        }
        )
    }

    return $results
}

function Invoke-ParallelTasks {

    param(
        [Parameter(Mandatory)]
        [array]$Tasks,

        [int]$ThrottleLimit = [Environment]::ProcessorCount
    )

    $jobs = foreach ($task in $Tasks) {

        Start-ThreadJob -Name $task.Name -ArgumentList $task -ScriptBlock {

            param($task)

            try {

                $params = if ($task.Arguments) {
                    $task.Arguments
                }
                else {
                    @{}
                }

                $result = & $task.ScriptPath @params

                if ($null -eq $result) {
                    throw "Script returned no object."
                }

                [PSCustomObject]@{
                    Name    = $task.Name
                    Success = $true
                    Result  = $result
                    Error   = $null
                }

            }
            catch {

                [PSCustomObject]@{
                    Name    = $task.Name
                    Success = $false
                    Result  = $null
                    Error   = $_ | Out-String
                }

            }

        }

    }

    Wait-Job $jobs | Out-Null

    $results = foreach ($job in $jobs) {

        $r = Receive-Job $job

        Remove-Job $job

        $r

    }

    return $results
}
