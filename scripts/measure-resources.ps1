[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateRange(1, 2147483647)][int]$AppProcessId,
    [ValidateRange(1, 86400)][int]$DurationSeconds = 600,
    [ValidateRange(0.1, 3600)][double]$IntervalSeconds = 2,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '../artifacts/performance'),
    [string]$Scenario = 'unspecified'
)
$ErrorActionPreference = 'Stop'
# Read-only observation. No application launch, termination or system setting changes.
$root = Get-Process -Id $AppProcessId -ErrorAction Stop
$rootStarted = $root.StartTime.ToUniversalTime()
$logicalCores = [Environment]::ProcessorCount
$runId = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssfffZ')
$destination = [IO.Path]::GetFullPath($OutputDirectory)
$null = New-Item -ItemType Directory -Path $destination -Force
$csvPath = Join-Path $destination "$runId-samples.csv"
$jsonPath = Join-Path $destination "$runId-summary.json"
$timer = [Diagnostics.Stopwatch]::StartNew()
$tracked = @{}
$previous = @{}
$previousAt = $null
$previousUtc = $null
$firstSampleAt = $null
$rows = [Collections.Generic.List[object]]::new()
$warnings = [Collections.Generic.List[string]]::new()
$stopReason = 'duration_reached'
$rootKey = "$AppProcessId/$($rootStarted.Ticks)"
$tracked[$rootKey] = $true

while ($true) {
    $sampleAt = $timer.Elapsed.TotalSeconds
    $sampleUtc = [DateTime]::UtcNow
    # Identity includes creation time: an OS-reused PID never inherits CPU history.
    $inventory = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, CreationDate, Name)
    $candidates = @{}
    foreach ($entry in $inventory) {
        if ($null -eq $entry.CreationDate) { continue }
        $born = ([DateTime]$entry.CreationDate).ToUniversalTime()
        $key = "$($entry.ProcessId)/$($born.Ticks)"
        # CIM timestamp precision can differ from Process.StartTime: root uses exact .NET value below.
        if ([int]$entry.ProcessId -eq $AppProcessId -and [Math]::Abs(($born - $rootStarted).TotalMilliseconds) -lt 1) { $key = $rootKey }
        $candidates[$key] = [pscustomobject]@{ Key = $key; Pid = [int]$entry.ProcessId; ParentPid = [int]$entry.ParentProcessId; Born = $born; Name = $entry.Name }
    }
    $selected = @{}
    foreach ($key in $candidates.Keys) { if ($tracked.ContainsKey($key)) { $selected[$key] = $candidates[$key] } }
    # Include descendants recursively, including WebView2 processes; preserve known orphans.
    do {
        $added = $false
        foreach ($key in $candidates.Keys) {
            if ($selected.ContainsKey($key)) { continue }
            $child = $candidates[$key]
            foreach ($parent in @($selected.Values)) {
                if ($child.ParentPid -eq $parent.Pid -and $child.Born -ge $parent.Born) {
                    $selected[$key] = $child
                    $tracked[$key] = $true
                    $added = $true
                    break
                }
            }
        }
    } while ($added)
    if ($selected.Count -eq 0) { $stopReason = 'process_tree_exited'; break }
    $privateCounters = @{}
    foreach ($counter in @(Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Select-Object IDProcess, WorkingSetPrivate)) {
        $privateCounters[[int]$counter.IDProcess] = [double]$counter.WorkingSetPrivate
    }
    # Timestamp CPU collection after potentially slow CIM queries, not before them.
    $sampleAt = $timer.Elapsed.TotalSeconds
    $sampleUtc = [DateTime]::UtcNow
    if ($null -eq $firstSampleAt) { $firstSampleAt = $sampleAt }
    $current = @{}
    $details = [Collections.Generic.List[object]]::new()
    $cpuDelta = 0.0
    $privateBytes = 0.0
    $cpuComplete = $null -ne $previousAt
    $memoryComplete = $true
    foreach ($entry in $selected.Values) {
        try {
            $process = Get-Process -Id $entry.Pid -ErrorAction Stop
            $actualStart = $process.StartTime.ToUniversalTime()
            if ([Math]::Abs(($actualStart - $entry.Born).TotalMilliseconds) -ge 1) { throw 'Process identity changed during sample' }
            $cpuSeconds = $process.TotalProcessorTime.TotalSeconds
            $delta = $null
            if ($previous.ContainsKey($entry.Key)) {
                $delta = $cpuSeconds - $previous[$entry.Key]
                if ($delta -lt 0) { $delta = $null; $cpuComplete = $false }
            } elseif ($null -ne $previousUtc -and $actualStart -ge $previousUtc) {
                $delta = $cpuSeconds # New process born within this interval.
            } else { $cpuComplete = $false }
            if ($null -ne $delta) { $cpuDelta += $delta }
            $memory = $null
            if ($privateCounters.ContainsKey($entry.Pid)) {
                # Check again after reading the counter; PID reuse must not attach to this identity.
                $check = Get-Process -Id $entry.Pid -ErrorAction Stop
                if ($check.StartTime.ToUniversalTime() -ne $actualStart) { throw 'Process identity changed during counter read' }
                $memory = $privateCounters[$entry.Pid]
                $privateBytes += $memory
            } else { $memoryComplete = $false }
            $current[$entry.Key] = $cpuSeconds
            $details.Add([pscustomobject]@{ pid = $entry.Pid; parentPid = $entry.ParentPid; name = $entry.Name; startedUtc = $actualStart.ToString('o'); cpuSeconds = $cpuSeconds; intervalCpuSeconds = $delta; privateWorkingSetBytes = $memory })
        } catch {
            $cpuComplete = $false
            $memoryComplete = $false
            $warnings.Add("$($sampleUtc.ToString('o')) PID $($entry.Pid): process exited, changed identity, or was inaccessible")
        }
    }
    foreach ($key in $previous.Keys) { if (-not $current.ContainsKey($key)) { $cpuComplete = $false } }
    $elapsed = if ($null -ne $previousAt) { $sampleAt - $previousAt } else { 0 }
    $cpuPercent = if ($cpuComplete -and $elapsed -gt 0) { 100 * $cpuDelta / $elapsed / $logicalCores } else { $null }
    $memoryMiB = if ($memoryComplete) { $privateBytes / 1MB } else { $null }
    $row = [pscustomobject]@{
        timestampUtc = $sampleUtc.ToString('o'); elapsedSeconds = $sampleAt - $firstSampleAt; intervalSeconds = $elapsed
        processCount = $current.Count; cpuPercent = $cpuPercent; cpuComplete = $cpuComplete
        privateWorkingSetMiB = $memoryMiB; memoryComplete = $memoryComplete
        processesJson = ConvertTo-Json -InputObject @($details.ToArray()) -Compress -Depth 4
    }
    $rows.Add($row)
    $row | Export-Csv -LiteralPath $csvPath -NoTypeInformation -Append -Encoding utf8
    $previous = $current; $previousAt = $sampleAt; $previousUtc = $sampleUtc
    if (($sampleAt - $firstSampleAt) -ge $DurationSeconds) { break }
    $remaining = $DurationSeconds - ($timer.Elapsed.TotalSeconds - $firstSampleAt)
    if ($remaining -gt 0) { Start-Sleep -Milliseconds ([int](1000 * [Math]::Min($IntervalSeconds, $remaining))) }
}
$validCpu = @($rows | Where-Object cpuComplete)
$validMemory = @($rows | Where-Object memoryComplete)
$cpuWall = ($validCpu | Measure-Object intervalSeconds -Sum).Sum
$weightedCpu = 0.0
foreach ($row in $validCpu) { $weightedCpu += $row.cpuPercent * $row.intervalSeconds }
$summary = [ordered]@{
    scenario = $Scenario; rootPid = $AppProcessId; rootStartedUtc = $rootStarted.ToString('o')
    os = [Environment]::OSVersion.VersionString; logicalProcessors = $logicalCores
    requestedDurationSeconds = $DurationSeconds; requestedIntervalSeconds = $IntervalSeconds
    observedSeconds = $(if ($null -ne $previousAt) { $previousAt - $firstSampleAt } else { 0 }); sampleCount = $rows.Count; stopReason = $stopReason
    tenMinuteObservation = ($null -ne $previousAt -and ($previousAt - $firstSampleAt) -ge 600)
    validCpuIntervals = $validCpu.Count; incompleteCpuSamples = $rows.Count - $validCpu.Count
    cpuCoveredSeconds = $cpuWall
    cpuMeanPercent = $(if ($cpuWall -gt 0) { $weightedCpu / $cpuWall } else { $null })
    cpuPeakPercent = ($validCpu | Measure-Object cpuPercent -Maximum).Maximum
    privateWorkingSetMeanMiB = ($validMemory | Measure-Object privateWorkingSetMiB -Average).Average
    privateWorkingSetPeakMiB = ($validMemory | Measure-Object privateWorkingSetMiB -Maximum).Maximum
    incompleteMemorySamples = $rows.Count - $validMemory.Count
    samplesCsv = $csvPath; warnings = @($warnings.ToArray())
    limitations = @('Polling cannot observe a process born and exited entirely between samples.', 'Exited processes may leave incomplete CPU intervals; those intervals are excluded from CPU statistics.', 'Private working set is sampled from Windows performance counters; this is not private committed bytes.', 'Record hardware, build mode, WebView2 version and scenario separately; no acceptance verdict is inferred.')
}
$summary | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $jsonPath -Encoding utf8
Write-Output $jsonPath
