param([string]$ArchiveName='DeviceCenter-source-0.1.0', [string]$BinaryRelativePath='releases/portable-polish/DeviceCenter.exe')
$ErrorActionPreference = 'Stop'
if($ArchiveName -notmatch '^[A-Za-z0-9.-]+$' -or $BinaryRelativePath -notmatch '^releases/portable[-a-z]*/DeviceCenter\.exe$'){throw 'Invalid archive name or binary reference'}
$workspace = Split-Path $PSScriptRoot -Parent
$stage = Join-Path (Join-Path $workspace '.tools') ('source-stage-' + [guid]::NewGuid().ToString('N'))
$archiveDirectory = Join-Path $workspace 'releases/source'
New-Item -ItemType Directory -Force $archiveDirectory | Out-Null
$archive = Join-Path $archiveDirectory ($ArchiveName+'.zip')
$externalManifest = Join-Path $archiveDirectory ($ArchiveName+'.manifest.json')
$top = @('.gitignore','LICENSE','AGENTS.md','README.md','THIRD_PARTY_NOTICES.md','index.html','package.json','package-lock.json','svelte.config.js','tsconfig.json','vite.config.ts')
$native = @('Cargo.toml','Cargo.lock','build.rs','tauri.conf.json','LICENSE','LICENSE-ATK','UPSTREAM.md','build-windows.ps1','validate-installer.ps1','validate-lifecycle.ps1','archive-source.ps1')
$files = @($top | ForEach-Object { Join-Path $workspace $_ })
$files += @($native | ForEach-Object { Join-Path $PSScriptRoot $_ })
foreach ($folder in @('src','src-tauri/src','src-tauri/capabilities','src-tauri/icons','docs','tests','scripts','public','assets/icons-v2')) {
  $path = Join-Path $workspace $folder
  if (Test-Path -LiteralPath $path) { $files += @(Get-ChildItem -LiteralPath $path -File -Recurse | ForEach-Object FullName) }
}
$files = @($files | Sort-Object -Unique)
New-Item -ItemType Directory -Path $stage | Out-Null
$manifest = @()
foreach ($file in $files) {
  if (!(Test-Path -LiteralPath $file -PathType Leaf)) { throw "缺少白名单输入：$file" }
  $relative = $file.Substring($workspace.Length + 1).Replace('\','/')
  if ($relative -match '(?i)(^|/)(\.tools|node_modules|target|references|portable[^/]*|artifacts)(/|$)|(^|/)\.env($|\.)|\.(dpapi|pem|key|csv|exe|dll|zip)$') { throw "拒绝非源码路径：$relative" }
  $originalHash = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash
  $destination = Join-Path $stage $relative
  New-Item -ItemType Directory -Force (Split-Path $destination -Parent) | Out-Null
  $sanitized = $false
  if ($relative -match '\.md$') {
    $original = [IO.File]::ReadAllText($file)
    $text = $original.Replace($workspace,'<WORKSPACE>').Replace($workspace.Replace('\','/'),'<WORKSPACE>')
    $text = [regex]::Replace($text,'(?i)[A-Z]:[\\/]Users[\\/][^\\/\s`"<>]+','<USER_PROFILE>')
    $text = [regex]::Replace($text,'(?i)\bPID\s*[:=]?\s*\d+','PID[omitted]')
    $text = [regex]::Replace($text,'(?i)\bsession\s*\d+','session[omitted]')
    $text = [regex]::Replace($text,'(?<![\d.])(?:10(?:\.\d{1,3}){3}|192\.168(?:\.\d{1,3}){2}|172\.(?:1[6-9]|2\d|3[01])(?:\.\d{1,3}){2})(?![\d.])','<LOCAL_DEVICE_IP>')
    $text = [regex]::Replace($text,'(?i)\b(?:[0-9a-f]{2}:){5}[0-9a-f]{2}\b','<DEVICE_MAC>')
    $text = [regex]::Replace($text,'install-smoke-[a-f0-9]{32}','install-smoke-<isolated-test>')
    $text = [regex]::Replace($text,'device-center-target-[A-F0-9]{12}','device-center-target-<workspace-hash>')
    $sanitized = $text -ne $original
    [IO.File]::WriteAllText($destination,$text,[Text.UTF8Encoding]::new($false))
  } else { Copy-Item -LiteralPath $file -Destination $destination }
  $manifest += [pscustomobject]@{Path=$relative;Bytes=(Get-Item -LiteralPath $destination).Length;SHA256=(Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash;WorkspaceSHA256=$originalHash;DocumentationSanitized=$sanitized}
}
foreach ($entry in $manifest) {
  if ((Get-FileHash -LiteralPath (Join-Path $workspace $entry.Path) -Algorithm SHA256).Hash -ne $entry.WorkspaceSHA256) { throw "输入在归档期间发生变更：$($entry.Path)" }
}
$binary = Join-Path $workspace $BinaryRelativePath
$binaryHash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
$payload = [ordered]@{
  FormatVersion=1;CreatedUtc=[DateTime]::UtcNow.ToString('o');Product='Device Center';Version='0.1.0'
  BinaryReference=@{Path=$BinaryRelativePath;SHA256=$binaryHash;Included=$false}
  Relationship='Current workspace source snapshot associated with the referenced binary by hash. No Git commit binding or bit-identical rebuild guarantee.'
  Privacy='Explicit allowlist. No runtime data, credentials, PID samples, downloaded website JavaScript, reference repositories, tools, node_modules, targets or binary distributions. Markdown copies redact local paths, runtime IDs, private device IPs and MAC addresses; product source bytes are unchanged.'
  ManifestCoverage='Files below only; generated SOURCE_MANIFEST.json and SOURCE_RELATION.md are metadata and intentionally excluded from their own content-hash list.'
  Files=$manifest
}
$json = $payload | ConvertTo-Json -Depth 8
[IO.File]::WriteAllText((Join-Path $stage 'SOURCE_MANIFEST.json'),$json,[Text.UTF8Encoding]::new($false))
$relation = @"
# Source snapshot for Device Center 0.1.0

This archive is associated with $BinaryRelativePath :

SHA256: $binaryHash

The executable is not included. SOURCE_MANIFEST.json records every allowlisted file's archived hash and its workspace hash. Application source/configuration/lock files are copied unchanged. Documentation copies redact local profile/workspace paths, runtime process/session IDs, private device addresses and installation-test directory IDs. Workspace originals were not edited.

This is a current workspace snapshot, not a Git-commit-pinned release or a complete capture of every build input. It does not prove that rebuilding this ZIP will produce bit-identical bytes. Toolchains, dependency availability, host SDK/runtime, build environment, timestamps and Tauri's bundle-type patching can affect output. Existing test and package evidence retains its stated scope; no build, running-app interaction, login or device operation was performed to create this archive.

Build instructions: README.md and src-tauri/build-windows.ps1. Lock files: package-lock.json and src-tauri/Cargo.lock. Required Windows native toolchain and WebView2 dependencies are not bundled. References and downloaded vendor scripts are intentionally excluded; notices retain provenance and GPL/MIT license texts are included. Do not treat historical documentation as new acceptance evidence.
"@
[IO.File]::WriteAllText((Join-Path $stage 'SOURCE_RELATION.md'),$relation,[Text.UTF8Encoding]::new($false))
Add-Type -AssemblyName System.IO.Compression.FileSystem
if (Test-Path -LiteralPath $archive) { throw '源码归档已存在，先核验现有产物，避免静默覆盖。' }
[IO.Compression.ZipFile]::CreateFromDirectory($stage,$archive)
$zip = [IO.Compression.ZipFile]::OpenRead($archive)
try {
  $allowed = @($manifest | ForEach-Object Path) + @('SOURCE_MANIFEST.json','SOURCE_RELATION.md')
  $entries = @($zip.Entries | ForEach-Object FullName)
  if ($entries.Count -ne $allowed.Count -or @($entries | Where-Object { $_ -notin $allowed }).Count -ne 0) { throw 'ZIP清单与白名单不一致。' }
  if (@($entries | Where-Object { $_ -match '(?i)(^|/)(\.tools|node_modules|target|references|portable[^/]*|artifacts)(/|$)|(^|/)\.env($|\.)|\.(dpapi|pem|key|csv|exe|dll|zip)$|^[A-Z]:|\.\.' }).Count -ne 0) { throw 'ZIP清单含禁止路径。' }
  foreach ($entry in $zip.Entries) {
    $expected = $manifest | Where-Object Path -eq $entry.FullName | Select-Object -First 1
    if ($expected) {
      $stream=$entry.Open();$sha=[Security.Cryptography.SHA256]::Create()
      try { $actual=[BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-','') } finally { $stream.Dispose();$sha.Dispose() }
      if ($actual -ne $expected.SHA256) { throw "ZIP内容哈希不匹配：$($entry.FullName)" }
    }
  }
} finally { $zip.Dispose() }
[IO.File]::WriteAllText($externalManifest,$json,[Text.UTF8Encoding]::new($false))
[pscustomobject]@{Archive=$archive;Bytes=(Get-Item -LiteralPath $archive).Length;SHA256=(Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash;PayloadFileCount=$manifest.Count;ZipEntryCount=$entries.Count;SanitizedDocumentationCount=@($manifest | Where-Object DocumentationSanitized).Count;BinaryReferenceSHA256=$binaryHash;Manifest=$externalManifest;ZipAllowlistAndContentHashesVerified=$true} | ConvertTo-Json
