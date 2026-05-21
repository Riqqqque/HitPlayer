param(
  [string] $Version = "8.1.1",
  [string] $BuildName = "essentials_build"
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$binaryDir = Join-Path $root 'src-tauri\binaries'
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "hitplayer-ffmpeg-$Version"
$zipPath = Join-Path $tempRoot "ffmpeg-$Version-$BuildName.zip"
$extractDir = Join-Path $tempRoot 'extract'
$url = "https://github.com/GyanD/codexffmpeg/releases/download/$Version/ffmpeg-$Version-$BuildName.zip"

New-Item -ItemType Directory -Force -Path $binaryDir, $tempRoot | Out-Null
if (Test-Path -LiteralPath $extractDir) {
  Remove-Item -LiteralPath $extractDir -Recurse -Force
}

Write-Host "Downloading FFmpeg $Version..."
Invoke-WebRequest -Uri $url -OutFile $zipPath

Write-Host "Extracting FFmpeg..."
Expand-Archive -LiteralPath $zipPath -DestinationPath $extractDir -Force

$ffmpeg = Get-ChildItem -LiteralPath $extractDir -Recurse -Filter ffmpeg.exe | Select-Object -First 1
$ffprobe = Get-ChildItem -LiteralPath $extractDir -Recurse -Filter ffprobe.exe | Select-Object -First 1

if (-not $ffmpeg -or -not $ffprobe) {
  throw "Could not find ffmpeg.exe and ffprobe.exe in the downloaded archive."
}

Copy-Item -LiteralPath $ffmpeg.FullName -Destination (Join-Path $binaryDir 'ffmpeg.exe') -Force
Copy-Item -LiteralPath $ffprobe.FullName -Destination (Join-Path $binaryDir 'ffprobe.exe') -Force
Copy-Item -LiteralPath $ffmpeg.FullName -Destination (Join-Path $binaryDir 'ffmpeg-x86_64-pc-windows-msvc.exe') -Force
Copy-Item -LiteralPath $ffprobe.FullName -Destination (Join-Path $binaryDir 'ffprobe-x86_64-pc-windows-msvc.exe') -Force

Write-Host "FFmpeg sidecars are ready in $binaryDir"
