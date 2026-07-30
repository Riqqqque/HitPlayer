param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$')]
  [string] $Version
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Update-FirstJsonVersion {
  param(
    [Parameter(Mandatory = $true)]
    [string] $Path,
    [Parameter(Mandatory = $true)]
    [string] $Version
  )

  $content = Get-Content -LiteralPath $Path -Raw
  $regex = [regex]'("version"\s*:\s*)"[^"]+"'
  $updated = $regex.Replace(
    $content,
    { param($match) "$($match.Groups[1].Value)`"$Version`"" },
    1
  )
  [System.IO.File]::WriteAllText($Path, $updated, $utf8NoBom)
}

Push-Location $root
try {
  npm version $Version --no-git-tag-version --allow-same-version | Out-Null
}
finally {
  Pop-Location
}

Update-FirstJsonVersion -Path (Join-Path $root 'src-tauri\tauri.conf.json') -Version $Version

$cargoPath = Join-Path $root 'src-tauri\Cargo.toml'
$cargo = Get-Content -LiteralPath $cargoPath -Raw
$cargo = $cargo -replace '(?m)^version = ".*"\r?$', "version = `"$Version`""
[System.IO.File]::WriteAllText($cargoPath, $cargo, $utf8NoBom)

Write-Host "HitPlayer version set to $Version"
