# Prepare Clean Distribution Package for RustGet v2.0
$ErrorActionPreference = "Stop"
$distRoot = "RustGet-v2.0"

if (Test-Path $distRoot) {
    Remove-Item $distRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $distRoot | Out-Null
New-Item -ItemType Directory -Path (Join-Path $distRoot "Portable") | Out-Null
New-Item -ItemType Directory -Path (Join-Path $distRoot "Browser-Extensions") | Out-Null
New-Item -ItemType Directory -Path (Join-Path $distRoot "Browser-Extensions\Chrome-Edge-Brave-Opera") | Out-Null
New-Item -ItemType Directory -Path (Join-Path $distRoot "Browser-Extensions\Mozilla-Firefox") | Out-Null

# 1. Copy Installer
Write-Host "[*] Copying Installer..."
Copy-Item "installer\dist\RustGet-Setup-2.0.0.exe" -Destination (Join-Path $distRoot "RustGet-Setup-2.0.0.exe")

# 2. Copy Portable Dir
Write-Host "[*] Copying Portable binaries..."
Copy-Item "installer\dist\RustGet-v2.0.0-windows-x64\*" -Destination (Join-Path $distRoot "Portable") -Recurse
if (Test-Path "target\release\libunwind.dll") {
    Copy-Item "target\release\libunwind.dll" -Destination (Join-Path $distRoot "Portable\libunwind.dll") -Force
}

# Copy Manifests to Portable root
Copy-Item "extension\com.rustget.downloader.chrome.json" -Destination (Join-Path $distRoot "Portable\com.rustget.downloader.chrome.json")
Copy-Item "extension\com.rustget.downloader.edge.json" -Destination (Join-Path $distRoot "Portable\com.rustget.downloader.edge.json")
Copy-Item "extension-firefox\com.rustget.downloader.firefox.json" -Destination (Join-Path $distRoot "Portable\com.rustget.downloader.firefox.json")

# 3. Build and Copy Extensions
Write-Host "[*] Packaging and Copying Browser Extensions..."
& powershell -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "build_xpi.ps1")
Copy-Item "extension\*" -Destination (Join-Path $distRoot "Browser-Extensions\Chrome-Edge-Brave-Opera") -Recurse
Copy-Item "extension-firefox\*" -Destination (Join-Path $distRoot "Browser-Extensions\Mozilla-Firefox") -Recurse
Copy-Item "extension-firefox\rustget-firefox.xpi" -Destination (Join-Path $distRoot "Portable\rustget-firefox.xpi")

# 4. Create Batch Scripts & Documentation
Write-Host "[*] Generating setup scripts and user guide..."
$regBatContent = @"
@echo off
title RustGet - Register Browser Integration
echo ========================================================
echo   RustGet v2.0 - Browser Native Messaging Setup
echo ========================================================
echo.
cd /d "%~dp0"

if not exist "rustget-host.exe" (
    echo [ERROR] rustget-host.exe not found in this folder!
    pause
    exit /b 1
)

echo Registering RustGet with Google Chrome, Microsoft Edge, and Mozilla Firefox...
rustget-host.exe --register "%~dp0com.rustget.downloader.chrome.json"

echo.
echo ========================================================
echo [SUCCESS] Browser integration registered successfully!
echo Please restart your browser(s) to apply the changes.
echo ========================================================
echo.
pause
"@

$unregBatContent = @"
@echo off
title RustGet - Unregister Browser Integration
echo ========================================================
echo   RustGet v2.0 - Remove Browser Integration
echo ========================================================
echo.
cd /d "%~dp0"

rustget-host.exe --unregister

echo.
echo ========================================================
echo [SUCCESS] Browser integration removed.
echo ========================================================
echo.
pause
"@

$instructionsContent = @"
================================================================================
                       ⚡ RustGet v2.0 Download Manager
                 Fast Multi-Segment Downloader for Windows 11
================================================================================

Welcome to RustGet v2.0! This package contains everything needed to install,
integrate with your browsers, and immediately start downloading files and videos
at maximum network speed.

--------------------------------------------------------------------------------
OPTION 1: AUTOMATIC INSTALLER (Recommended for most users)
--------------------------------------------------------------------------------
1. Double-click "RustGet-Setup-2.0.0.exe".
2. Follow the setup wizard (click Next -> Install).
3. That's it!
   - RustGet will be installed to your system.
   - Start Menu and Desktop shortcuts will be created.
   - Browser Native Messaging bridge will be automatically registered in the registry.

--------------------------------------------------------------------------------
OPTION 2: PORTABLE MODE (No installation required)
--------------------------------------------------------------------------------
1. Open the "Portable" folder (or extract "RustGet-v2.0.0-Portable.zip").
2. Double-click "register-browsers.bat" to register the browser bridge.
3. Run "rustget-gui.exe" anytime you want to use RustGet!

--------------------------------------------------------------------------------
HOW TO ADD THE BROWSER EXTENSION
--------------------------------------------------------------------------------

A) FOR GOOGLE CHROME, MICROSOFT EDGE, BRAVE, OR OPERA:
   1. Open your browser and go to the Extensions page:
      - Chrome: chrome://extensions
      - Edge:   edge://extensions
      - Brave:  brave://extensions
   2. Turn ON "Developer mode" (toggle switch in the top-right or top-left corner).
   3. Click the "Load unpacked" button.
   4. Select the folder:
      "Browser-Extensions\Chrome-Edge-Brave-Opera"
   5. Done! The RustGet icon will appear in your browser toolbar.

B) FOR MOZILLA FIREFOX:
   Note: Mozilla Firefox by design unloads "Temporary Add-ons" when the browser closes.
   To install the extension permanently so it never vanishes on restart:

   METHOD 1 (Permanent via Firefox Developer Edition, Nightly, ESR, Floorp, or LibreWolf):
   1. Open Firefox and go to: about:config
   2. Search for: xpinstall.signatures.required
   3. Double-click it to set it to: false
   4. Open: about:addons (or press Ctrl + Shift + A)
   5. Click the Gear icon (⚙) at the top right and select "Install Add-on From File..."
   6. Choose "Browser-Extensions\Mozilla-Firefox\rustget-firefox.xpi" (or drag and drop it into Firefox).
   7. Click "Add" when prompted. It is now permanently installed!

   METHOD 2 (Permanent for Standard Consumer Firefox Release):
   Standard Firefox requires all permanent add-ons to be signed by Mozilla's automated system (100% free, takes 2 minutes):
   1. Go to: https://addons.mozilla.org/developers/addon/submit/upload-unlisted
   2. Sign in with your free Firefox account.
   3. Upload the package "Browser-Extensions\Mozilla-Firefox\rustget-firefox.xpi".
   4. Select "On your own (unlisted)" (this keeps your extension private).
   5. Mozilla's automated system signs it in ~2 minutes and gives you a signed .xpi download.
   6. Drag and drop that signed .xpi into Firefox. It will install permanently and never vanish!

   METHOD 3 (Quick Testing in Any Firefox):
   1. Go to: about:debugging#/runtime/this-firefox
   2. Click "Load Temporary Add-on..." and select "Browser-Extensions\Mozilla-Firefox\manifest.json".

--------------------------------------------------------------------------------
KEY FEATURES & USAGE TIPS
--------------------------------------------------------------------------------
• IDM-Style Download Dialog:
  Whenever you click a download link in your browser, RustGet intercepts the
  download and pops up the "Download File Info" window. Click "Start Download"
  to download immediately, or "Download Later" to save it to your queue.

• Minimize to System Tray:
  Clicking the close button [X] on the window minimizes RustGet to the system
  tray, keeping it quietly ready in the background. Left-click the tray icon to
  bring the window back. To completely exit, right-click the tray icon and
  choose "Exit".

• Video & YouTube Downloads:
  When watching videos on YouTube or other websites, RustGet displays a small,
  convenient "Download Video" floating button directly on the video player.
  Clicking it automatically sends the video stream to RustGet. You can also
  copy any YouTube URL to your clipboard or click "+ Add URL" in RustGet.

• Up to 32 Parallel Connections:
  Files are split into high-speed concurrent segments with automatic resumption
  if your network disconnects.

--------------------------------------------------------------------------------
Support & Open Source:
RustGet is licensed under the MIT License.
Enjoy your lightning-fast downloads!
================================================================================
"@

[System.IO.File]::WriteAllText((Join-Path $distRoot "Portable\register-browsers.bat"), $regBatContent)
[System.IO.File]::WriteAllText((Join-Path $distRoot "Portable\unregister-browsers.bat"), $unregBatContent)
[System.IO.File]::WriteAllText((Join-Path $distRoot "Portable\HOW_TO_INSTALL.txt"), $instructionsContent)
[System.IO.File]::WriteAllText((Join-Path $distRoot "HOW_TO_INSTALL.txt"), $instructionsContent)

# 5. Copy Portable ZIP and re-compress with batch scripts
Write-Host "[*] Packaging Portable ZIP..."
$zipPath = Join-Path $distRoot "RustGet-v2.0.0-Portable.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path (Join-Path $distRoot "Portable\*") -DestinationPath $zipPath -Force

# 5. Create Master All-In-One ZIP
Write-Host "[*] Creating Master Distribution ZIP..."
$masterZip = "RustGet-v2.0.zip"
if (Test-Path $masterZip) { Remove-Item $masterZip -Force }
Compress-Archive -Path (Join-Path $distRoot "*") -DestinationPath $masterZip -Force

Write-Host "[OK] Distribution folder and archives populated successfully!"
