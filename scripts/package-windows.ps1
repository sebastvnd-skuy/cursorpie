# Builds target\bundle\CursorPie-windows.zip containing cursorpie.exe (tray app, no console window).
# Usage (PowerShell, MSVC build tools installed):  scripts\package-windows.ps1
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"(.*)"' | Select-Object -First 1).Matches.Groups[1].Value

cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$out = "target\bundle"
$stage = Join-Path $out "CursorPie"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item "target\release\cursorpie.exe" (Join-Path $stage "cursorpie.exe")
Copy-Item LICENSE, README.md $stage

$zip = Join-Path $out "CursorPie-windows.zip"
Remove-Item -Force $zip -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip
Write-Host "Built $zip (version $version)"
