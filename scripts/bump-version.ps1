param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$')]
  [string] $Version
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Update-JsonVersion {
  param(
    [Parameter(Mandatory = $true)]
    [string] $Path,
    [Parameter(Mandatory = $true)]
    [string] $Version
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $updated = $content -replace '("version"\s*:\s*)"[^"]+"', "`$1`"$Version`""
  [System.IO.File]::WriteAllText($Path, $updated, $utf8NoBom)
}

Update-JsonVersion -Path (Join-Path $root 'package.json') -Version $Version
Update-JsonVersion -Path (Join-Path $root 'src-tauri\tauri.conf.json') -Version $Version

$cargoPath = Join-Path $root 'src-tauri\Cargo.toml'
$cargo = Get-Content -LiteralPath $cargoPath -Raw
$cargo = $cargo -replace '(?m)^version = ".*"$', "version = `"$Version`""
[System.IO.File]::WriteAllText($cargoPath, $cargo, $utf8NoBom)

npm install --package-lock-only --silent

Write-Host "HitPlayer version set to $Version"
