param([string]$InstallerPath = (Join-Path (Split-Path $PSScriptRoot -Parent) 'releases/DeviceCenter_0.1.0_x64-setup.exe'))
$ErrorActionPreference = 'Stop'
$workspace = Split-Path $PSScriptRoot -Parent
$productKey = 'HKCU:\Software\device-center\Device Center'
$manufacturerKey = 'HKCU:\Software\device-center'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Device Center'
$appIdKey = 'HKCU:\Software\Classes\AppUserModelId\local.device-center.app'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$profileId = 'local.device-center.app'
function Autostart-Exists {
  $entry = Get-ItemProperty -LiteralPath $runKey -ErrorAction SilentlyContinue
  return ($null -ne $entry -and $null -ne $entry.PSObject.Properties['Device Center'])
}
function Protected-Metadata {
  $data = [ordered]@{}
  foreach ($folder in @((Join-Path $env:APPDATA $profileId),(Join-Path $env:LOCALAPPDATA $profileId))) {
    $data[$folder] = Test-Path -LiteralPath $folder
  }
  # Inspect metadata only; never read DPAPI contents, browser data or cloud sessions.
  foreach ($name in @('lamp.dpapi','config.json','settings.json')) {
    $path = Join-Path (Join-Path $env:APPDATA $profileId) $name
    $file = Get-Item -LiteralPath $path -ErrorAction SilentlyContinue
    $data[$path] = if ($file) { "$($file.Length):$($file.LastWriteTimeUtc.Ticks)" } else { 'absent' }
  }
  foreach ($path in @((Join-Path ([Environment]::GetFolderPath('DesktopDirectory')) 'Device Center.lnk'),(Join-Path ([Environment]::GetFolderPath('Programs')) 'Device Center.lnk'))) {
    $file = Get-Item -LiteralPath $path -ErrorAction SilentlyContinue
    $data[$path] = if ($file) { "$($file.Length):$($file.LastWriteTimeUtc.Ticks)" } else { 'absent' }
  }
  return ($data | ConvertTo-Json -Compress)
}
function Assert-Unchanged {
  if ((Protected-Metadata) -ne $beforeMetadata) { throw '检测到应用配置或快捷方式元数据并发变化，已停止；不推断变化来源。' }
  if (Autostart-Exists) { throw '检测到同名自启项并发出现，已停止。' }
  foreach ($entry in $protectedProcesses) {
    $process = Get-Process -Id $entry.Id -ErrorAction SilentlyContinue
    if (!$process -or $process.StartTime.Ticks -ne $entry.StartTicks) { throw '现用便携进程状态变化，已停止。' }
  }
}
foreach ($key in @($productKey,$uninstallKey,$appIdKey,'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Device Center')) {
  if (Test-Path -LiteralPath $key) { throw "已有相关安装注册项，不执行隔离测试：$key" }
}
if (Autostart-Exists) { throw '已有同名自启项，不执行隔离测试。' }
$webview = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -Name pv -ErrorAction SilentlyContinue
if (!$webview.pv) { throw '未确认已有WebView2，不执行可能涉及运行时安装的测试。' }
$manufacturerExisted = Test-Path -LiteralPath $manufacturerKey
$protectedProcesses = @(Get-Process -Name DeviceCenter -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$workspace\releases\portable*\DeviceCenter.exe" } | ForEach-Object { [pscustomobject]@{Id=$_.Id;StartTicks=$_.StartTime.Ticks} })
$beforeMetadata = Protected-Metadata
$testDir = Join-Path (Join-Path $workspace '.tools') ('install-smoke-' + [guid]::NewGuid().ToString('N'))
$testDir = [IO.Path]::GetFullPath($testDir)
$allowedRoot = [IO.Path]::GetFullPath((Join-Path $workspace '.tools')) + [IO.Path]::DirectorySeparatorChar
if (!$testDir.StartsWith($allowedRoot,[StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $testDir)) { throw '测试目录不是全新项目隔离路径。' }
Assert-Unchanged
$installer = (Resolve-Path -LiteralPath $InstallerPath).Path
$installerHash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
$installProcess = Start-Process -FilePath $installer -ArgumentList "/S /NS /UPDATE /D=$testDir" -WindowStyle Hidden -Wait -PassThru
if ($installProcess.ExitCode -ne 0) { throw "隔离安装失败，退出码 $($installProcess.ExitCode)，保留现场 $testDir" }
Assert-Unchanged
$installed = Get-ItemProperty -LiteralPath $uninstallKey
if ($installed.InstallLocation.Trim('"') -ne $testDir -or (Get-Item -LiteralPath $productKey).GetValue('') -ne $testDir) { throw '安装注册目标不匹配，停止并保留现场。' }
foreach ($name in @('device-center.exe','WebView2Loader.dll','LICENSE','LICENSE-ATK','UPSTREAM.md','THIRD_PARTY_NOTICES.md','uninstall.exe')) {
  if (!(Test-Path -LiteralPath (Join-Path $testDir $name))) { throw "安装缺少文件 $name，保留现场。" }
}
if (Get-Process -Name device-center -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq (Join-Path $testDir 'device-center.exe') }) { throw '测试目录中的应用意外启动，停止。' }
Assert-Unchanged
# /UPDATE makes the app-data and Autostart removal branches unreachable.
# _?= keeps the uninstaller process synchronous; it is the last unquoted parameter.
$uninstallProcess = Start-Process -FilePath (Join-Path $testDir 'uninstall.exe') -ArgumentList "/S /UPDATE _?=$testDir" -WindowStyle Hidden -Wait -PassThru
if ($uninstallProcess.ExitCode -ne 0) { throw "隔离卸载失败，退出码 $($uninstallProcess.ExitCode)，保留现场。" }
Assert-Unchanged
if (Test-Path -LiteralPath $uninstallKey) { throw '卸载注册项仍存在，停止并保留现场。' }
# /UPDATE intentionally leaves the installation-location key. Remove only our new key.
if (Test-Path -LiteralPath $productKey) {
  $key = Get-Item -LiteralPath $productKey
  if ($key.GetValue('') -ne $testDir -or $key.SubKeyCount -ne 0) { throw '产品注册项有并发变化，停止。' }
  Remove-Item -LiteralPath $productKey
}
if (!$manufacturerExisted -and (Test-Path -LiteralPath $manufacturerKey)) {
  $key = Get-Item -LiteralPath $manufacturerKey
  if ($key.SubKeyCount -eq 0 -and $key.ValueCount -eq 0) { Remove-Item -LiteralPath $manufacturerKey }
}
# Remove only the known self-locked uninstaller; never recursively delete this directory.
if (Test-Path -LiteralPath (Join-Path $testDir 'uninstall.exe')) { Remove-Item -LiteralPath (Join-Path $testDir 'uninstall.exe') }
if (Test-Path -LiteralPath $testDir) {
  if (@(Get-ChildItem -LiteralPath $testDir -Force).Count -ne 0) { throw '隔离目录存在其他残留，保留现场不递归删除。' }
  Remove-Item -LiteralPath $testDir
}
Assert-Unchanged
[pscustomobject]@{
  InstallerSHA256=$installerHash;InstallExitCode=$installProcess.ExitCode;UninstallExitCode=$uninstallProcess.ExitCode
  TestDirectory=$testDir;TestDirectoryRemoved=(!(Test-Path -LiteralPath $testDir));ProtectedMetadataUnchanged=$true
  ExistingPortablePids=@($protectedProcesses | ForEach-Object Id);ExistingPortableProcessesUnchanged=$true
  ProductRegistryRestored=(!(Test-Path -LiteralPath $productKey));UninstallRegistryRestored=(!(Test-Path -LiteralPath $uninstallKey))
  SameNameAutostartAbsent=(!(Autostart-Exists));InstalledAppWasNotStarted=$true
} | ConvertTo-Json -Depth 4
