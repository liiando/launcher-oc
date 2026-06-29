# Build & deploy the launcher to the clean game folder.
# Usage: .\build.ps1
param(
    [switch]$Release = $true
)

$ErrorActionPreference = "Stop"
$project = "D:\ONly\ONLYCLIMB MOD 2026 BY UNREZNAN\launcher"
$target  = "D:\ONly\onlyclimb_clean\onlyclimbtogether\Binaries\Win64"

$profile = if ($Release) { "--release" } else { "" }
cargo build $profile --manifest-path "$project\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "Build failed" }

$src = Join-Path $project "target\release\onlyclimb-launcher.exe"
Copy-Item $src $target -Force
Write-Host "Deployed to $target" -ForegroundColor Green
