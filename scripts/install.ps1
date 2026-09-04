# Degunk Windows Installer
# Repository: https://github.com/AbdullahHassan192/degunk

$ErrorActionPreference = 'Stop'

$Owner = "AbdullahHassan192"
$Repo = "degunk"
$Asset = "degunk-windows-x86_64.zip"
$DownloadUrl = "https://github.com/$Owner/$Repo/releases/latest/download/$Asset"

Write-Host "Installing degunk for Windows (x86_64)..." -ForegroundColor Cyan

# Target directory in local app data
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\degunk"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$TempZip = Join-Path ([System.IO.Path]::GetTempPath()) "degunk.zip"

try {
    Write-Host "Downloading latest release..." -ForegroundColor DarkGray
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempZip -UseBasicParsing

    Write-Host "Extracting binary..." -ForegroundColor DarkGray
    Expand-Archive -Path $TempZip -DestinationPath $InstallDir -Force
} finally {
    if (Test-Path $TempZip) {
        Remove-Item -Force $TempZip
    }
}

$BinaryPath = Join-Path $InstallDir "degunk.exe"
if (!(Test-Path $BinaryPath)) {
    Write-Error "Failed to install degunk.exe"
    exit 1
}

# Ensure InstallDir is on User Path
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$PathEntries = $UserPath -split ';' | Where-Object { $_ -ne "" }

if ($PathEntries -notcontains $InstallDir) {
    Write-Host "Adding $InstallDir to user PATH..." -ForegroundColor DarkGray
    $NewUserPath = if ([string]::IsNullOrEmpty($UserPath)) { $InstallDir } else { "$UserPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $NewUserPath, "User")
    $env:Path += ";$InstallDir"
}

Write-Host "✓ Successfully installed degunk to $BinaryPath" -ForegroundColor Green
Write-Host ""
Write-Host "Open a new terminal or run 'degunk' to begin scanning your workspace." -ForegroundColor White
