param([Parameter(Mandatory=$true)][string]$Target)
$ErrorActionPreference='Stop'
$image=[Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes((Join-Path $Target 'device-center.exe')))
if(!$image.Contains('LIFECYCLE_VALIDATION_ONLY')) { throw 'Refuse to run production executable as lifecycle mock.' }
$image=$null
$run=Join-Path $env:TEMP ('device-center-lifecycle-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $run | Out-Null
Copy-Item -LiteralPath (Join-Path $Target 'device-center.exe') -Destination $run
Copy-Item -LiteralPath (Join-Path $Target 'WebView2Loader.dll') -Destination $run
$previousRoot=$env:DEVICE_CENTER_LIFECYCLE_ROOT
$previousProfile=$env:WEBVIEW2_USER_DATA_FOLDER
function ProtectedState {
  $running=@(Get-Process -Name device-center -ErrorAction SilentlyContinue | Where-Object {$_.Path -like '*portable*'} | Sort-Object Id | ForEach-Object {@{pid=$_.Id;start=$_.StartTime.ToFileTimeUtc();path=$_.Path}})
  $files=@(foreach($base in @($env:APPDATA,$env:LOCALAPPDATA)) {foreach($name in @('lamp.dpapi','config.json','settings.json')) {$f=Get-Item -LiteralPath (Join-Path $base ('local.device-center.app/'+$name)) -ErrorAction SilentlyContinue;if($f){@{path=$f.FullName;length=$f.Length;modified=$f.LastWriteTimeUtc.ToFileTimeUtc()}}}})
  @{running=$running;files=$files}|ConvertTo-Json -Compress -Depth 5
}
$protectedBefore=ProtectedState
$protectedBefore|Set-Content -LiteralPath (Join-Path $run 'protected-before.json')
try {
$env:DEVICE_CENTER_LIFECYCLE_ROOT=$run
$env:WEBVIEW2_USER_DATA_FOLDER=Join-Path $run 'webview-profile'
$proc=Start-Process -FilePath (Join-Path $run 'device-center.exe') -WorkingDirectory $run -PassThru
$record=Join-Path $run 'process-samples.jsonl'
Write-Host "Validation PID=$($proc.Id) Root=$run"
for($i=0;$i -lt 110;$i++) {
  $all=@(Get-CimInstance Win32_Process -Filter "name='device-center.exe' OR name='msedgewebview2.exe'" | Select-Object ProcessId,ParentProcessId,Name)
  $ids=[Collections.Generic.HashSet[uint32]]::new(); [void]$ids.Add([uint32]$proc.Id)
  do {$added=$false; foreach($entry in $all) {if($ids.Contains([uint32]$entry.ParentProcessId) -and $ids.Add([uint32]$entry.ProcessId)){$added=$true}}}while($added)
  $tree=@($all | Where-Object {$ids.Contains([uint32]$_.ProcessId)} | ForEach-Object { $p=Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue; @{pid=$_.ProcessId;parent=$_.ParentProcessId;name=$_.Name;privateBytes=$p.PrivateMemorySize64} })
  @{at=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds();processes=$tree}|ConvertTo-Json -Compress -Depth 5|Add-Content -LiteralPath $record
  $proc.Refresh();if($proc.HasExited){break};Start-Sleep -Seconds 1
}
$proc.Refresh()
if(!$proc.HasExited){throw "Validation timeout; own validation PID $($proc.Id) remains; no user process touched."}
$events=[IO.File]::ReadAllText((Join-Path $run 'events.jsonl'))
$required=@('dirty_close_cancelled_main_alive','mock_cloud_pending','cloud_close_prompt_visible','cloud_cancelled_and_main_destroyed','zero_windows_alive_20s','main_cache_restored','compact_cache_restored','compact_native_size_360_440','compact_native_close_requested','compact_native_close_completed','all_windows_destroyed_alive_20s','complete')
foreach($event in $required){if(!$events.Contains('"'+$event+'"')){throw "Missing event $event; evidence $run"}}
if($events.Contains('FAIL_') -or $proc.ExitCode -ne 0){throw "Lifecycle failure; evidence $run"}
Write-Host "PASS lifecycle events and OS process samples: $run"
$protectedAfter=ProtectedState
$protectedAfter|Set-Content -LiteralPath (Join-Path $run 'protected-after.json')
if($protectedBefore -ne $protectedAfter){throw 'Protected production process/config metadata changed; inspect metadata evidence.'}
Write-Host 'PASS protected production process/config metadata unchanged'
} finally {
 $env:DEVICE_CENTER_LIFECYCLE_ROOT=$previousRoot
 $env:WEBVIEW2_USER_DATA_FOLDER=$previousProfile
}
