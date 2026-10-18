[CmdletBinding()]
param(
  [switch]$SkipTests,
  [switch]$SkipUiAudit,
  [switch]$DebugBuild,
  [switch]$CleanDebug
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest

$Root = $PSScriptRoot

$NodeHome = if ($env:NODE_HOME) { $env:NODE_HOME } else { 'E:\node' }
$NodeExe = Join-Path $NodeHome 'node.exe'
Add-Type -AssemblyName System.IO.Compression.FileSystem
function New-PortableZip {
    param([string]$SourceDir, [string]$Destination)
    if (Test-Path -LiteralPath $Destination) { Remove-Item -LiteralPath $Destination -Force }
    # Compress-Archive rejects the 1973 timestamp shipped in WebView2Loader.dll
    [System.IO.Compression.ZipFile]::CreateFromDirectory(
        $SourceDir, $Destination, [System.IO.Compression.CompressionLevel]::Optimal, $false)
    if (-not (Test-Path -LiteralPath $Destination)) { throw "Portable ZIP was not produced: $Destination" }
    Write-Host "portable: $Destination" -ForegroundColor Green
}

$CargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path -LiteralPath (Join-Path $CargoBin 'cargo.exe')) {
  $env:Path = $CargoBin + ';' + $env:Path
}
$TargetDir = Join-Path $Root 'target\release'
$DistDir = Join-Path $Root 'dist'
$AppName = 'ApiCheckinHelper'
$PackageDir = Join-Path $DistDir ($AppName + '-win-x64')

function Write-Step([string]$Text) {
  Write-Host ''
  Write-Host "==> $Text" -ForegroundColor Cyan
}

function Invoke-Step([string]$Label, [string]$Command, [string[]]$Arguments) {
  Write-Host ("    " + $Label)
  & $Command @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "$Label failed with exit code $LASTEXITCODE."
  }
}

function Remove-Tree([string]$Path) {
  if (-not (Test-Path -LiteralPath $Path)) { return }
  $resolved = [System.IO.Path]::GetFullPath($Path)
  if (-not $resolved.StartsWith([System.IO.Path]::GetFullPath($Root), [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to delete outside the project: $resolved"
  }
  [System.IO.Directory]::Delete($resolved, $true)
}

Push-Location $Root
try {
  Write-Step 'Format'
  Invoke-Step 'cargo fmt --all -- --check' 'cargo' @('fmt', '--all', '--', '--check')

  Write-Step 'Lint (warnings are errors)'
  Invoke-Step 'cargo clippy --all-targets -- -D warnings' 'cargo' @('clippy', '--all-targets', '--', '-D', 'warnings')

  if (-not $SkipTests) {
    Write-Step 'Tests'
    Invoke-Step 'cargo test' 'cargo' @('test')
  }

  Write-Step 'Release build'
  Invoke-Step 'cargo build --release' 'cargo' @('build', '--release')

  Write-Step 'Package'
  Remove-Tree $PackageDir
  New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null

  $exe = Join-Path $TargetDir ($AppName + '.exe')
  if (-not (Test-Path -LiteralPath $exe)) { throw "Release executable is missing: $exe" }
  Copy-Item -LiteralPath $exe -Destination (Join-Path $PackageDir ($AppName + '.exe')) -Force -ErrorAction Stop

  $uninstaller = Join-Path $TargetDir 'uninstall.exe'
  if (-not (Test-Path -LiteralPath $uninstaller)) { throw "Uninstaller executable is missing: $uninstaller" }
  Copy-Item -LiteralPath $uninstaller -Destination (Join-Path $PackageDir 'uninstall.exe') -Force -ErrorAction Stop

  $loader = Get-ChildItem -LiteralPath (Join-Path $Root 'target') -Recurse -Filter 'WebView2Loader.dll' -File -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match 'x64' } |
    Select-Object -First 1
  if ($null -eq $loader) { throw 'WebView2Loader.dll was not produced by the build.' }
  Copy-Item -LiteralPath $loader.FullName -Destination (Join-Path $PackageDir 'WebView2Loader.dll') -Force -ErrorAction Stop

  Copy-Item -LiteralPath (Join-Path $Root 'app.ico') -Destination (Join-Path $PackageDir 'app.ico') -Force -ErrorAction Stop
  Copy-Item -LiteralPath (Join-Path $Root 'README.md') -Destination (Join-Path $PackageDir 'README.md') -Force -ErrorAction Stop
  Copy-Item -LiteralPath (Join-Path $Root 'LICENSE') -Destination (Join-Path $PackageDir 'LICENSE') -Force -ErrorAction Stop

  $webTarget = Join-Path $PackageDir 'Ui\Web'
  New-Item -ItemType Directory -Path $webTarget -Force | Out-Null
  Get-ChildItem -Path (Join-Path $Root 'Ui\Web') -File |
    Where-Object { $_.Name -ne 'source.css' } |
    ForEach-Object {
      Copy-Item -LiteralPath $_.FullName -Destination $webTarget -Force -ErrorAction Stop
    }
  $expected = @('index.html', 'styles.css', 'app.js', 'i18n.js', 'lucide.min.js')
  foreach ($name in $expected) {
    if (-not (Test-Path -LiteralPath (Join-Path $webTarget $name))) {
      throw "Front-end asset is missing from the package: $name"
    }
  }

  Write-Step 'Inno Setup'
  $iscc = 'C:\Program Files (x86)\Inno Setup 7\ISCC.exe'
  if (-not (Test-Path -LiteralPath $iscc)) { $iscc = 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe' }
  if (-not (Test-Path -LiteralPath $iscc)) {
    Write-Warning 'Inno Setup was not found; skipping the installer step.'
  } else {
    & $iscc --quiet (Join-Path $Root 'installer\ApiCheckinHelper.iss')
    if ($LASTEXITCODE -ne 0) { throw 'Inno Setup reported a compilation error.' }
    Write-Host ("installer: " + (Join-Path $Root 'dist\ApiCheckinHelper-Setup.exe')) -ForegroundColor Green
  }

  if (-not $SkipUiAudit) {
    $audit = Join-Path $Root 'tools\ui-audit\run.js'
    if (Test-Path -LiteralPath $audit) {
      $playwright = Join-Path $NodeHome 'node_modules\playwright-core\package.json'
      if (-not (Test-Path -LiteralPath $playwright)) {
        Write-Warning 'playwright-core is not installed; skipping the UI audit.'
      } else {
        Write-Step 'UI audit'
        & $NodeExe $audit
        if ($LASTEXITCODE -ne 0) { throw 'UI audit reported violations.' }
      }
    }
  }

  Write-Step 'WiX MSI'
  $wix = Join-Path $env:USERPROFILE '.dotnet\tools\wix.exe'
  if (-not (Test-Path -LiteralPath $wix)) {
    Write-Warning 'The WiX tool was not found; skipping the MSI step.'
  } else {
    $msiFile = Join-Path $Root 'dist\ApiCheckinHelper.msi'
    & $wix build (Join-Path $Root 'installer\Package.wxs') -ext WixToolset.UI.wixext -arch x64 -pdbtype none -d ProductVersion=1.0.0 -d ("AppSource=" + $PackageDir) -o $msiFile
    if ($LASTEXITCODE -ne 0) { throw 'WiX reported a compilation error.' }
    if (-not (Test-Path -LiteralPath $msiFile)) { throw "MSI was not produced: $msiFile" }
    Write-Host "installer: $msiFile" -ForegroundColor Green
  }

  Write-Step 'Portable ZIP'
  New-PortableZip -SourceDir $PackageDir -Destination (Join-Path $Root 'dist\ApiCheckinHelper-portable-win-x64.zip')

  Write-Step 'Artifacts'
  Get-ChildItem -LiteralPath $PackageDir -Recurse -File | Sort-Object FullName |
    ForEach-Object { '{0,-52} {1,12:N0} bytes' -f $_.FullName.Substring($PackageDir.Length + 1), $_.Length }
  Write-Host ''
  Write-Host 'Build finished.' -ForegroundColor Green
}
finally {
  Pop-Location
}
if ($CleanDebug) {
    $debugDir = Join-Path $PSScriptRoot 'target\debug'
    if (Test-Path -LiteralPath $debugDir) {
        Remove-Item -LiteralPath $debugDir -Recurse -Force
        Write-Host "cleaned: $debugDir" -ForegroundColor Green
    }
}