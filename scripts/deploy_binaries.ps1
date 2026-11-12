# Cosmo Downloader - Unified Deployment & In-Place Update Script
[CmdletBinding()]
param(
    [ValidateSet("Dev", "Release")]
    [string]$Mode = "Release",

    [string]$InstallPath = "",

    [string]$InstallerUrl = "",

    [string]$ExpectedSha256 = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RootDir = Split-Path -Parent $ScriptDir

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "     Cosmo Downloader In-Place Update / Deployment        " -ForegroundColor Cyan
Write-Host "     Mode: $Mode                                          " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Downloaded Installer Mode (if InstallerUrl provided)
if ($InstallerUrl -ne "") {
    Write-Host "[*] Downloading installer from $InstallerUrl..." -ForegroundColor Yellow
    $tempInstaller = Join-Path $env:TEMP "CosmoDownloader-Setup.exe"
    Invoke-WebRequest -Uri $InstallerUrl -OutFile $tempInstaller

    if ($ExpectedSha256 -ne "") {
        Write-Host "[*] Verifying installer SHA256 checksum..." -ForegroundColor Yellow
        $actualSha256 = (Get-FileHash -Path $tempInstaller -Algorithm SHA256).Hash.ToLowerInvariant()
        $expected = $ExpectedSha256.Trim().ToLowerInvariant()
        if ($actualSha256 -ne $expected) {
            Remove-Item $tempInstaller -Force -ErrorAction SilentlyContinue
            throw "Security error: Installer SHA256 hash mismatch! Expected: $expected, Actual: $actualSha256"
        }
        Write-Host "[OK] SHA256 checksum verified: $actualSha256" -ForegroundColor Green
    }

    Write-Host "[*] Running installer in silent upgrade mode..." -ForegroundColor Yellow
    $p = Start-Process -FilePath $tempInstaller -ArgumentList "/VERYSILENT /NORESTART /SUPPRESSMSGBOXES" -PassThru -Wait
    if ($p.ExitCode -ne 0) {
        throw "Installer exited with failure code $($p.ExitCode)"
    }
    Remove-Item $tempInstaller -Force -ErrorAction SilentlyContinue
    Write-Host "[OK] In-place upgrade completed successfully via installer!" -ForegroundColor Green
    return
}

# 2. Terminate running processes safely with timeout
$processNames = @("CosmoDownloader", "cosmodownloader-host", "cosmodownloader-cli", "rustget-gui", "rustget-host", "rustget")
$running = Get-Process -Name $processNames -ErrorAction SilentlyContinue
if ($running) {
    Write-Host "[*] Stopping running Cosmo Downloader processes..." -ForegroundColor Yellow
    $running | Stop-Process -ErrorAction SilentlyContinue
    try {
        $running | Wait-Process -Timeout 5 -ErrorAction Stop
    } catch {
        Write-Warning "Processes did not exit within 5s; terminating forcefully..."
        $running | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 1
    }
    Write-Host "[OK] Background processes stopped." -ForegroundColor Green
}

# 3. Locate compiled source binaries
$targetSubDir = if ($Mode -eq "Dev") { "target\debug" } else { "target\release" }
$sourceDir = Join-Path $RootDir $targetSubDir
if (-not (Test-Path $sourceDir)) {
    # Fallback check
    $altDir = Join-Path $RootDir "target\release"
    if (Test-Path $altDir) {
        $sourceDir = $altDir
    } else {
        throw "Build directory not found at $sourceDir. Please compile the workspace first."
    }
}

$mainExe = Join-Path $sourceDir "CosmoDownloader.exe"
$cliExe = Join-Path $sourceDir "cosmodownloader-cli.exe"
$hostExe = Join-Path $sourceDir "cosmodownloader-host.exe"

$sourceBinaries = @($mainExe, $cliExe, $hostExe)
foreach ($bin in $sourceBinaries) {
    if (-not (Test-Path $bin)) {
        throw "Required binary missing in ${sourceDir}: $(Split-Path -Leaf $bin). Please run cargo build."
    }
}

# 4. Resolve installed application directory dynamically
$targetInstallDir = $InstallPath
if ($targetInstallDir -eq "") {
    $regKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{D37E7492-3580-4D3B-A924-E8251C9B71C8}_is1"
    if (Test-Path $regKey) {
        $prop = Get-ItemProperty -Path $regKey -Name "InstallLocation" -ErrorAction SilentlyContinue
        if ($prop -and $prop.InstallLocation) {
            $targetInstallDir = $prop.InstallLocation
        }
    }
}

if ($targetInstallDir -eq "" -or (-not (Test-Path $targetInstallDir))) {
    $standardPath = Join-Path $env:LOCALAPPDATA "Programs\Cosmo Downloader"
    $compatPath = Join-Path $env:LOCALAPPDATA "Programs\CosmoDownloader"
    if (Test-Path $standardPath) {
        $targetInstallDir = $standardPath
    } elseif (Test-Path $compatPath) {
        $targetInstallDir = $compatPath
    } else {
        $targetInstallDir = $standardPath
        New-Item -ItemType Directory -Path $targetInstallDir -Force | Out-Null
    }
}

Write-Host "[*] Target installation directory: $targetInstallDir" -ForegroundColor Cyan

# 5. Atomic copy with .old renaming and rollback on failure
$renamedFiles = @()
try {
    Write-Host "[*] Deploying binaries with atomic rollback protection..." -ForegroundColor Yellow
    foreach ($src in $sourceBinaries) {
        $leaf = Split-Path -Leaf $src
        $dest = Join-Path $targetInstallDir $leaf
        if (Test-Path $dest) {
            $old = "$dest.old"
            if (Test-Path $old) { Remove-Item $old -Force }
            Rename-Item -Path $dest -NewName "$leaf.old" -Force
            $renamedFiles += @{ Old = $old; Original = $dest }
        }
        Copy-Item -Path $src -Destination $dest -Force
        Write-Host "  -> Updated $leaf" -ForegroundColor DarkGray
    }

    # Backward compatibility aliases for CLI and legacy calls
    $legacyCli = Join-Path $targetInstallDir "rustget.exe"
    $legacyHost = Join-Path $targetInstallDir "rustget-host.exe"
    Copy-Item -Path $cliExe -Destination $legacyCli -Force
    Copy-Item -Path $hostExe -Destination $legacyHost -Force

    # Copy application icon used by shortcuts
    $iconSrc = Join-Path $RootDir "icon.ico"
    if (Test-Path $iconSrc) {
        Copy-Item -Path $iconSrc -Destination (Join-Path $targetInstallDir "icon.ico") -Force
    }

    # Copy extension folders and manifests
    $extSrc = Join-Path $RootDir "browser-extensions"
    $chromeDest = Join-Path $targetInstallDir "Browser-Extensions\Chrome-Edge-Brave-Opera"
    $ffDest = Join-Path $targetInstallDir "Browser-Extensions\Mozilla-Firefox"

    New-Item -ItemType Directory -Path $chromeDest -Force | Out-Null
    New-Item -ItemType Directory -Path $ffDest -Force | Out-Null

    Copy-Item (Join-Path $extSrc "Chrome-Edge-Brave-Opera\*") -Destination $chromeDest -Recurse -Force
    Copy-Item (Join-Path $extSrc "Mozilla-Firefox\*") -Destination $ffDest -Recurse -Force

    # Copy root native messaging manifests into install root
    Get-ChildItem -Path (Join-Path $extSrc "Chrome-Edge-Brave-Opera") -Filter "*.json" | ForEach-Object {
        Copy-Item $_.FullName -Destination (Join-Path $targetInstallDir $_.Name) -Force
    }

    # On success, clean up .old backup files
    foreach ($item in $renamedFiles) {
        if (Test-Path $item.Old) {
            Remove-Item $item.Old -Force -ErrorAction SilentlyContinue
        }
    }
    Write-Host "[OK] Binaries and extensions successfully deployed to $targetInstallDir" -ForegroundColor Green
} catch {
    Write-Error "Deployment failed: $_. Initiating automatic rollback..."
    foreach ($item in $renamedFiles) {
        if (Test-Path $item.Old) {
            Copy-Item -Path $item.Old -Destination $item.Original -Force -ErrorAction SilentlyContinue
            Remove-Item $item.Old -Force -ErrorAction SilentlyContinue
        }
    }
    throw
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Deployment Complete! You may now launch Cosmo Downloader." -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan
