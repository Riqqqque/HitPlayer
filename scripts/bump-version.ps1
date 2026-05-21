param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$')]
  [string] $Version
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Update-JsonVersion {
  param(
    [Parameter(Mandatory = $true)]
    [string] $Path,
    [Parameter(Mandatory = $true)]
    [string] $Version
  )

  $json = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
  $json.version = $Version
  $json | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $Path -Encoding UTF8
}

Update-JsonVersion -Path (Join-Path $root 'package.json') -Version $Version
Update-JsonVersion -Path (Join-Path $root 'src-tauri\tauri.conf.json') -Version $Version

$cargoPath = Join-Path $root 'src-tauri\Cargo.toml'
$cargo = Get-Content -LiteralPath $cargoPath -Raw
$cargo = $cargo -replace '(?m)^version = ".*"$', "version = `"$Version`""
Set-Content -LiteralPath $cargoPath -Value $cargo -Encoding UTF8

npm install --package-lock-only --silent

Write-Host "HitPlayer version set to $Version"
