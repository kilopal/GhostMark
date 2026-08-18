<#
.SYNOPSIS
Installs the ghostmark-clean agent skill globally.

.DESCRIPTION
This script compiles the GhostMark Rust CLI in release mode, creates the global skill directories for Antigravity (~/.gemini/config/skills) and Cursor (~/.cursor/skills), and copies the compiled binary and SKILL.md into them.
#>

$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$SkillDir = $PSScriptRoot

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "👻 Installing ghostmark-clean Skill" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 1. Compile the Rust CLI
Write-Host "`n[1/3] Compiling GhostMark CLI (Release Mode)..." -ForegroundColor Yellow
Set-Location $RepoRoot
try {
    cargo build --release -p ghostmark
} catch {
    Write-Host "❌ Compilation failed. Ensure Rust and Cargo are installed." -ForegroundColor Red
    exit 1
}

$BinaryPath = Join-Path $RepoRoot "target\release\ghostmark.exe"
if (-Not (Test-Path $BinaryPath)) {
    Write-Host "❌ Expected binary not found at $BinaryPath" -ForegroundColor Red
    exit 1
}

# 2. Define global skill paths
$UserProfile = [Environment]::GetFolderPath("UserProfile")
$GeminiSkillPath = Join-Path $UserProfile ".gemini\config\skills\ghostmark-clean"
$CursorSkillPath = Join-Path $UserProfile ".cursor\skills\ghostmark-clean"

# 3. Copy files
Write-Host "`n[2/3] Installing to Antigravity (~/.gemini)..." -ForegroundColor Yellow
if (-Not (Test-Path $GeminiSkillPath)) {
    New-Item -ItemType Directory -Force -Path $GeminiSkillPath | Out-Null
}
Copy-Item -Path $BinaryPath -Destination $GeminiSkillPath -Force
Copy-Item -Path (Join-Path $SkillDir "SKILL.md") -Destination $GeminiSkillPath -Force
Write-Host "✅ Installed at $GeminiSkillPath" -ForegroundColor Green

Write-Host "`n[3/3] Installing to Cursor (~/.cursor)..." -ForegroundColor Yellow
if (-Not (Test-Path $CursorSkillPath)) {
    New-Item -ItemType Directory -Force -Path $CursorSkillPath | Out-Null
}
Copy-Item -Path $BinaryPath -Destination $CursorSkillPath -Force
Copy-Item -Path (Join-Path $SkillDir "SKILL.md") -Destination $CursorSkillPath -Force
Write-Host "✅ Installed at $CursorSkillPath" -ForegroundColor Green

Write-Host "`n🎉 Installation complete! Agents can now use GhostMark locally.`n" -ForegroundColor Cyan
