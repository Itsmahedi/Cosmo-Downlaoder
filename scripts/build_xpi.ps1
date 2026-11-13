# Cosmo Downloader — Firefox Extension Packaging Script
# Generates an AMO-ready .zip archive and a companion .xpi file for Mozilla Firefox
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RootDir = Split-Path -Parent $ScriptDir
$ExtDir = Join-Path $RootDir "browser-extensions\Mozilla-Firefox"
$DistDir = Join-Path $RootDir "installer\dist"

if (-not (Test-Path $ExtDir)) {
    throw "Firefox extension directory not found at $ExtDir"
}

if (-not (Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir -Force | Out-Null
}

# Read version from manifest.json
$manifestPath = Join-Path $ExtDir "manifest.json"
$manifestJson = Get-Content $manifestPath -Raw | ConvertFrom-Json
$version = $manifestJson.version

$standardZipName = "firefox-extension.zip"
$zipName = "CosmoDownloader-Firefox-Extension-v$version.zip"
$xpiName = "CosmoDownloader-Firefox-Extension-v$version.xpi"

$outStandardZip = Join-Path $RootDir $standardZipName
$outDistZip = Join-Path $DistDir $zipName
$outDistXpi = Join-Path $DistDir $xpiName

$files = @(
    "manifest.json",
    "background.js",
    "content.js",
    "content.css",
    "popup.html",
    "popup.js",
    "popup.css",
    "icons/icon-16.png",
    "icons/icon-32.png",
    "icons/icon-48.png",
    "icons/icon-128.png"
)

# Verify all required files exist before packaging
foreach ($file in $files) {
    $filePath = Join-Path $ExtDir ($file -replace '/', '\')
    if (-not (Test-Path $filePath)) {
        throw "Required extension file missing: $filePath"
    }
}

$tempZip = Join-Path $env:TEMP $standardZipName
if (Test-Path $tempZip) { Remove-Item $tempZip -Force }

# Use tar.exe (bsdtar) to guarantee standard forward slash separators ('/')
& tar.exe -a -c -f $tempZip -C $ExtDir $files
if ($LASTEXITCODE -ne 0) {
    throw "tar.exe failed to package the extension archive."
}

Copy-Item $tempZip -Destination $outStandardZip -Force
Copy-Item $tempZip -Destination $outDistZip -Force
Copy-Item $tempZip -Destination $outDistXpi -Force
Remove-Item $tempZip -Force

Write-Host "==========================================================" -ForegroundColor Green
Write-Host " Firefox Manifest V3 Extension Packaged Successfully!     " -ForegroundColor Green
Write-Host " Ready for upload to addons.mozilla.org (AMO)             " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
Write-Host " Output File (Root)     : $outStandardZip"
Write-Host " Dist ZIP Package       : $outDistZip"
Write-Host " Dist XPI Package       : $outDistXpi"
Write-Host " Extension ID           : $($manifestJson.browser_specific_settings.gecko.id)"
Write-Host " Version                : $version"
Write-Host " Strict Min Version     : $($manifestJson.browser_specific_settings.gecko.strict_min_version)"
Write-Host " Android Min Version    : $($manifestJson.browser_specific_settings.gecko_android.strict_min_version)"
Write-Host " Data Collection        : None ($($manifestJson.browser_specific_settings.gecko.data_collection_permissions.required -join ', '))"
