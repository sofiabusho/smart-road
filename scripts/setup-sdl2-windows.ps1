# Install/configure SDL2 for smart-road on Windows (MSVC x64).
# Run once from PowerShell: .\scripts\setup-sdl2-windows.ps1

$ErrorActionPreference = "Stop"

$SdlRoot = Join-Path $env:USERPROFILE "SDL2"
$SdlVersionDir = Join-Path $SdlRoot "SDL2-2.30.10"
$ZipPath = Join-Path $env:TEMP "SDL2-devel-2.30.10-VC.zip"
$DownloadUrl = "https://github.com/libsdl-org/SDL/releases/download/release-2.30.10/SDL2-devel-2.30.10-VC.zip"

if (-not (Test-Path (Join-Path $SdlVersionDir "lib\x64\SDL2.lib"))) {
    Write-Host "Downloading SDL2 development package..."
    New-Item -ItemType Directory -Force -Path $SdlRoot | Out-Null
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing
    Expand-Archive -Path $ZipPath -DestinationPath $SdlRoot -Force
    Write-Host "Extracted to $SdlVersionDir"
} else {
    Write-Host "SDL2 already present at $SdlVersionDir"
}

$include = Join-Path $SdlVersionDir "include"
$lib = Join-Path $SdlVersionDir "lib\x64"
$bin = Join-Path $SdlVersionDir "lib\x64"

[Environment]::SetEnvironmentVariable("SDL2_INCLUDE_PATH", $include, "User")
[Environment]::SetEnvironmentVariable("SDL2_LIB_PATH", $lib, "User")

$userLib = [Environment]::GetEnvironmentVariable("LIB", "User")
if ($userLib -notlike "*$lib*") {
    if ([string]::IsNullOrEmpty($userLib)) {
        [Environment]::SetEnvironmentVariable("LIB", $lib, "User")
    } else {
        [Environment]::SetEnvironmentVariable("LIB", "$lib;$userLib", "User")
    }
}

$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$bin*") {
    [Environment]::SetEnvironmentVariable("PATH", "$bin;$userPath", "User")
}

# Apply to current session too
$env:SDL2_INCLUDE_PATH = $include
$env:SDL2_LIB_PATH = $lib
if ($env:PATH -notlike "*$bin*") {
    $env:PATH = "$bin;$env:PATH"
}

Write-Host ""
Write-Host "SDL2 configured for smart-road:"
Write-Host "  SDL2_INCLUDE_PATH = $include"
Write-Host "  SDL2_LIB_PATH     = $lib"
Write-Host "  PATH              += $bin"
Write-Host ""
Write-Host "Open a new terminal, then run:"
Write-Host "  cargo test"
Write-Host "  cargo run"
