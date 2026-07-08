#Requires -Version 5.1
<#
.SYNOPSIS
    Build Meta-Analyzer for Windows: NSIS installer + portable standalone.
.DESCRIPTION
    Runs `tauri build` (frontend + optimized Rust release + NSIS installer),
    then assembles the finished artifacts into .\release :

      - Meta-Analyzer_<ver>_x64-setup.exe   installer  -> stores data in %APPDATA%
      - Meta-Analyzer-Portable\  (+ .zip)   portable   -> stores data in .\data
                                                          next to the exe (self-contained)

    The same binary serves both: at startup the app uses a `data` folder next to
    the executable when present (portable), otherwise the per-user app-data dir
    (installed).

    Release optimization flags live in the workspace Cargo.toml [profile.release]
    (opt-level=s, lto, strip, panic=abort). WebView2 runtime is required
    (standard on Windows 10 21H2+ / Windows 11).
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProjectRoot = $PSScriptRoot
Set-Location $ProjectRoot

Write-Host "=== Meta-Analyzer Release Build ===" -ForegroundColor Cyan
Write-Host "Project: $ProjectRoot"
Write-Host ""

# --- Version: CalVer YY.M.D from the build date (auto-stamped, no manual bumping) ---
# SemVer requires plain numbers without leading zeros, so July 8 2026 -> 26.7.8.
# The value is chronological and drives the installer name + Windows version.
$now       = Get-Date
$Version   = '{0}.{1}.{2}' -f $now.ToString('yy'), [int]$now.Month, [int]$now.Day
$DateLabel = $now.ToString('yyyy-MM-dd')
# Date form used in the distributed file names (SemVer can't have dashes, so the
# installer/portable artifacts get the calendar label instead of "26.7.8").
$DateName  = $now.ToString('yy-MM-dd')
Write-Host "Version: $Version  ($DateLabel)" -ForegroundColor Cyan

function Set-JsonVersion([string]$RelPath, [string]$Ver) {
    $full = Join-Path $ProjectRoot $RelPath
    $text = [System.IO.File]::ReadAllText($full)
    # Replace only the first top-level "version": "..." (not nested dependency versions).
    $text = [regex]::new('"version"\s*:\s*"[^"]*"').Replace($text, ('"version": "{0}"' -f $Ver), 1)
    [System.IO.File]::WriteAllText($full, $text)
}
function Set-CargoVersion([string]$RelPath, [string]$Ver) {
    $full = Join-Path $ProjectRoot $RelPath
    $text = [System.IO.File]::ReadAllText($full)
    # ^version = "..." only matches the [package] line (dependency versions are inline).
    $text = [regex]::new('(?m)^version\s*=\s*"[^"]*"').Replace($text, ('version = "{0}"' -f $Ver), 1)
    [System.IO.File]::WriteAllText($full, $text)
}
Set-JsonVersion  'src-tauri\tauri.conf.json' $Version
Set-JsonVersion  'package.json'              $Version
Set-CargoVersion 'src-tauri\Cargo.toml'      $Version
Write-Host ""

# [1/4] Build frontend + Rust release + NSIS installer
Write-Host "[1/4] tauri build (frontend + release + installer)..." -ForegroundColor Yellow
Write-Host "      This may take several minutes on first run."
# tauri does not clean bundle/nsis, so old installers pile up. Remove stale ones
# first so we never accidentally collect a previous build's setup.exe.
Remove-Item (Join-Path $ProjectRoot 'target\release\bundle\nsis\*-setup.exe') -Force -ErrorAction SilentlyContinue
# node/cargo print progress to stderr; under $ErrorActionPreference='Stop'
# PowerShell 5.1 turns that into a fatal NativeCommandError. Relax it just for
# the native build call and gate on the real exit code instead.
$prevEAP = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
npx tauri build
$buildExit = $LASTEXITCODE
$ErrorActionPreference = $prevEAP
if ($buildExit -ne 0) {
    Write-Host "ERROR: tauri build failed (exit $buildExit)" -ForegroundColor Red
    exit 1
}

$TargetDir = Join-Path $ProjectRoot 'target\release'
$SourceExe = Join-Path $TargetDir 'meta-analyzer.exe'
$RulesDir  = Join-Path $TargetDir 'content_rules'
if (-not (Test-Path $SourceExe)) {
    Write-Host "ERROR: standalone exe not found at $SourceExe" -ForegroundColor Red
    exit 1
}

# [2/4] Reset the output folder
Write-Host ""
Write-Host "[2/4] Preparing .\release ..." -ForegroundColor Yellow
$OutputDir = Join-Path $ProjectRoot 'release'
if (Test-Path $OutputDir) { Remove-Item $OutputDir -Recurse -Force }
New-Item -ItemType Directory -Path $OutputDir | Out-Null

# [3/4] Collect the installer
Write-Host ""
Write-Host "[3/4] Collecting installer ..." -ForegroundColor Yellow
$NsisDir = Join-Path $TargetDir 'bundle\nsis'
# Pick the installer for THIS version (tauri names it Meta-Analyzer_<ver>_x64-setup.exe);
# fall back to the newest setup.exe if the name ever changes.
$Installer = Get-ChildItem $NsisDir -Filter ('*_{0}_*-setup.exe' -f $Version) -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $Installer) {
    $Installer = Get-ChildItem $NsisDir -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
}
if ($Installer) {
    # Tauri names the installer with the SemVer version (26.7.8); rename the copy
    # to the calendar form (26-07-08) for distribution.
    $InstallerOut = $Installer.Name -replace [regex]::Escape($Version), $DateName
    Copy-Item $Installer.FullName (Join-Path $OutputDir $InstallerOut) -Force
    Write-Host "      Installer: $InstallerOut"
} else {
    Write-Host "      WARN: no NSIS installer found (check bundle.active / targets)" -ForegroundColor DarkYellow
}

# [4/4] Assemble the portable package
#   A `data` folder next to the exe switches the app into portable mode, so we
#   pre-seed the content rules there. Everything the app writes stays in `data`.
Write-Host ""
Write-Host "[4/4] Assembling portable package ..." -ForegroundColor Yellow
$Portable     = Join-Path $OutputDir 'Meta-Analyzer-Portable'
$PortableData = Join-Path $Portable 'data'
New-Item -ItemType Directory -Path $PortableData -Force | Out-Null
Copy-Item $SourceExe (Join-Path $Portable 'Meta-Analyzer.exe') -Force
if (Test-Path $RulesDir) {
    Copy-Item $RulesDir (Join-Path $PortableData 'content_rules') -Recurse -Force
}

$Readme = @"
Meta-Analyzer - Portable edition

All data (settings, prompt profiles, content_rules, vocabulary) is stored in the
'data' folder next to Meta-Analyzer.exe. Nothing is written to your Windows user
profile - the app is fully self-contained.

Keep Meta-Analyzer.exe and the 'data' folder together. To reset everything,
delete the contents of 'data' (keep the 'data' folder itself to stay portable).

Requires the WebView2 runtime (standard on Windows 10 21H2+ / Windows 11):
https://developer.microsoft.com/en-us/microsoft-edge/webview2/
"@
Set-Content -Path (Join-Path $Portable 'README.txt') -Value $Readme -Encoding UTF8

# Zip the portable folder for distribution (calendar-dated name)
$Zip = Join-Path $OutputDir ('Meta-Analyzer-Portable_{0}.zip' -f $DateName)
Compress-Archive -Path $Portable -DestinationPath $Zip -Force

# Version marker
Set-Content -LiteralPath (Join-Path $OutputDir 'VERSION.txt') -Value "$Version  ($DateLabel)" -Encoding ascii

# Summary
Write-Host ""
Write-Host "=== Release complete ===" -ForegroundColor Green
Write-Host "Output: $OutputDir" -ForegroundColor Cyan
Get-ChildItem $OutputDir | ForEach-Object {
    if ($_.PSIsContainer) {
        Write-Host ("  [dir]  {0}" -f $_.Name)
    } else {
        $mb = [math]::Round($_.Length / 1MB, 1)
        Write-Host ("  {0,6} MB  {1}" -f $mb, $_.Name)
    }
}
Write-Host ""
Write-Host "Built: $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
