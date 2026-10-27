param([ValidateSet('test','check','hardware','kx-readonly','kx-keymap-readonly','cloud-unit','cloud-smoke','dev','release','installer','bundle','lifecycle')][string]$Action='test')
$ErrorActionPreference='Stop'
$project = Split-Path $PSScriptRoot -Parent
$sourceProject = $project
$sha = [System.Security.Cryptography.SHA256]::Create()
try { $workspaceHash = [BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($project))).Replace('-','').Substring(0,12) } finally { $sha.Dispose() }
$mappedDrive = $null
if ($project -match '[^\x00-\x7F]') {
  $mappedDrive = @('Z:','Y:','X:','W:','V:') | Where-Object { !(Test-Path ($_ + '\')) } | Select-Object -First 1
  if (!$mappedDrive) { throw 'GNU工具链需要无中文构建路径，未找到空闲临时盘符。' }
  & subst $mappedDrive $project
  if ($LASTEXITCODE -ne 0) { throw '临时构建盘符映射失败' }
  $project = $mappedDrive + '\'
}
$env:CARGO_HOME=Join-Path $project '.tools/cargo'
$env:RUSTUP_HOME=Join-Path $project '.tools/rustup'
$gccBin=Join-Path $project '.tools/winlibs/mingw64/bin'
if (!(Test-Path "$gccBin/gcc.exe")) { if ($mappedDrive) { & subst $mappedDrive /D }; throw '请先将完整 WinLibs x86_64 POSIX UCRT GCC 解压到 .tools/winlibs/mingw64；不使用 LLVM-only MinGW。' }
$env:PATH="$gccBin;$env:CARGO_HOME/bin;$env:PATH"
$loader = Get-ChildItem "$env:CARGO_HOME/registry/src/*/webview2-com-sys-*/x64/WebView2Loader.dll" -ErrorAction SilentlyContinue | Select-Object -Last 1
if (!$loader) {
  & cargo +stable-x86_64-pc-windows-gnu fetch --manifest-path (Join-Path $project 'src-tauri/Cargo.toml')
  if ($LASTEXITCODE -ne 0) { if ($mappedDrive) { & subst $mappedDrive /D }; throw '依赖下载失败' }
  $loader = Get-ChildItem "$env:CARGO_HOME/registry/src/*/webview2-com-sys-*/x64/WebView2Loader.dll" | Select-Object -Last 1
}
if ($loader) { $env:PATH = $loader.DirectoryName + ';' + $env:PATH }
$env:CC='gcc'
$env:AR='ar'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER='gcc'
$env:CARGO_TARGET_DIR=Join-Path $env:TEMP ('device-center-target-' + $workspaceHash)
if ($loader) {
  New-Item -ItemType Directory -Force (Join-Path $env:CARGO_TARGET_DIR 'debug/deps') | Out-Null
  Copy-Item -LiteralPath $loader.FullName -Destination (Join-Path $env:CARGO_TARGET_DIR 'debug/deps/WebView2Loader.dll') -Force
}
$env:CARGO_ENCODED_RUSTFLAGS='--sysroot'+[char]31+(Join-Path $env:RUSTUP_HOME 'toolchains/stable-x86_64-pc-windows-gnu')
$env:RUSTUP_TOOLCHAIN='stable-x86_64-pc-windows-gnu'
$env:TAURI_BUNDLER_CACHE_DIR=Join-Path $project '.tools/tauri-bundler'
Write-Host ('Native build cache: ' + $env:CARGO_TARGET_DIR)
Push-Location $project
try {
  switch ($Action) {
    'lifecycle' { & cargo +stable-x86_64-pc-windows-gnu build --manifest-path src-tauri/Cargo.toml --release --no-default-features --features custom-protocol,lifecycle-validation; if ($LASTEXITCODE -eq 0 -and $loader) { Copy-Item -LiteralPath $loader.FullName -Destination (Join-Path $env:CARGO_TARGET_DIR 'release/WebView2Loader.dll') -Force } }
    'cloud-unit' { & cargo +stable-x86_64-pc-windows-gnu test --manifest-path src-tauri/Cargo.toml --lib cloud::tests }
    'bundle' { Push-Location $sourceProject; try { & npx tauri bundle --bundles nsis } finally { Pop-Location } }
    'cloud-smoke' { & cargo +stable-x86_64-pc-windows-gnu test --manifest-path src-tauri/Cargo.toml --lib cloud::tests::anonymous_qr_begin_cancel_network_smoke -- --ignored --nocapture }
    'hardware' { & cargo +stable-x86_64-pc-windows-gnu test --manifest-path src-tauri/Cargo.toml hardware_read_once -- --ignored --nocapture }
    'kx-readonly' { & cargo +stable-x86_64-pc-windows-gnu test --manifest-path src-tauri/Cargo.toml --lib settings::kx_extensions::tests::hardware_kx75_readonly_performance_and_color_once -- --exact --ignored --nocapture }
    'kx-keymap-readonly' { & cargo +stable-x86_64-pc-windows-gnu test --manifest-path src-tauri/Cargo.toml --lib settings::kx_mapping::tests::hardware_kx75_readonly_keymap_layers_once -- --exact --ignored --nocapture }
    'test' { & cargo +stable-x86_64-pc-windows-gnu test --manifest-path src-tauri/Cargo.toml --lib }
    'check' { & cargo +stable-x86_64-pc-windows-gnu check --manifest-path src-tauri/Cargo.toml }
    'dev' { & cargo +stable-x86_64-pc-windows-gnu run --manifest-path src-tauri/Cargo.toml }
    { $_ -in @('release','installer') } { Push-Location $sourceProject; try { & npm run build; if ($LASTEXITCODE -ne 0) { throw '前端构建失败' } } finally { Pop-Location }; & cargo +stable-x86_64-pc-windows-gnu build --manifest-path src-tauri/Cargo.toml --release --no-default-features --features custom-protocol; if ($LASTEXITCODE -eq 0 -and $loader) { Copy-Item -LiteralPath $loader.FullName -Destination (Join-Path $env:CARGO_TARGET_DIR 'release/WebView2Loader.dll') -Force }; if ($Action -eq 'installer' -and $LASTEXITCODE -eq 0) { Push-Location $sourceProject; try { & npx tauri bundle --bundles nsis } finally { Pop-Location } } }
  }
  if ($LASTEXITCODE -ne 0) { throw "构建失败: $LASTEXITCODE" }
} finally { Pop-Location; if ($mappedDrive) { & subst $mappedDrive /D } }
