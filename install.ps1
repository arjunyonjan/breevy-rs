# breevy-rs installer (per-user, no admin)
$ErrorActionPreference = "Stop"

$src = Join-Path $PSScriptRoot "breevy-rs.exe"
if (-not (Test-Path $src)) { throw "breevy-rs.exe not found next to install.ps1" }

$dir = Join-Path $env:LOCALAPPDATA "BreevyRs"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
Copy-Item $src (Join-Path $dir "breevy-rs.exe") -Force

$data = Join-Path $env:APPDATA "Breevy\abbreviations.bvy"
if (-not (Test-Path $data)) {
    $data = Join-Path $dir "abbreviations.bvy"
    if (-not (Test-Path $data)) {
        throw "No abbreviations.bvy found. Copy your Breevy data file to $data first."
    }
}

$exe = Join-Path $dir "breevy-rs.exe"
$cmd = "`"$exe`" --hook `"$data`""
New-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" `
    -Name "breevy-rs" -Value $cmd -PropertyType String -Force | Out-Null

Write-Host "Installed: $exe"
Write-Host "Autostart: HKCU Run -> $cmd"
Write-Host "Starting now..."
Start-Process -FilePath $exe -ArgumentList "--hook", "`"$data`"" -WindowStyle Hidden
Write-Host "Done. Type an abbreviation and press Ctrl."
