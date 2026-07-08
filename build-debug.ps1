#Requires -Version 5.1
<#
.SYNOPSIS
    Build Meta-Analyzer in Debug mode AND launch it with live console output
.DESCRIPTION
    Builds the Svelte frontend and Rust backend in debug mode, then runs the
    resulting .exe attached to this PowerShell window so stdout/stderr (panics,
    println!, logs) are visible here. The window stays open after exit.

    Press Ctrl+C or close the app to return to the script; then press Enter to exit.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProjectRoot = $PSScriptRoot

Write-Host "=== Meta-Analyzer Debug Build & Run ===" -ForegroundColor Cyan
Write-Host "Project: $ProjectRoot"
Write-Host ""

# Step 1: Build frontend
Write-Host "[1/3] Building frontend (npm run build)..." -ForegroundColor Yellow
Set-Location $ProjectRoot
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Frontend build failed" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}
Write-Host "Frontend built." -ForegroundColor Green

# Step 2: Build Rust backend (debug) from workspace root
Write-Host ""
Write-Host "[2/3] Building Rust backend (cargo build)..." -ForegroundColor Yellow
Set-Location $ProjectRoot
cargo build --manifest-path src-tauri/Cargo.toml
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Rust build failed" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

$ExePath = "$ProjectRoot\target\debug\meta-analyzer.exe"
if (-not (Test-Path $ExePath)) {
    Write-Host "ERROR: Expected output not found at $ExePath" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

$Size = [math]::Round((Get-Item $ExePath).Length / 1MB, 1)
Write-Host "Built: $ExePath (${Size} MB)" -ForegroundColor Green

# Step 3: Launch with live output
Write-Host ""
Write-Host "[3/3] Launching app with attached console..." -ForegroundColor Yellow
Write-Host "--- App output below (stdout + stderr) ---" -ForegroundColor Cyan
Write-Host ""

# Enable Rust backtrace for clearer panics. Run directly so I/O streams are inherited.
$env:RUST_BACKTRACE = "1"
$env:RUST_LOG       = "info,meta_analyzer=debug,tauri=info"

& $ExePath
$ExitCode = $LASTEXITCODE

Write-Host ""
Write-Host "--- App exited (code: $ExitCode) ---" -ForegroundColor Cyan

if ($ExitCode -ne 0) {
    Write-Host "App exited with non-zero code. Look for panic messages above." -ForegroundColor Yellow
}

Write-Host ""
Read-Host "Press Enter to close this window"
