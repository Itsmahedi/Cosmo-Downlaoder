$ErrorActionPreference = "Stop"

$xpiOutput = "extension-firefox\rustget-firefox.xpi"
if (Test-Path $xpiOutput) { Remove-Item $xpiOutput -Force }

$tempDir = Join-Path $env:TEMP "rustget_ff_xpi"
if (Test-Path $tempDir) { Remove-Item $tempDir -Recurse -Force }
New-Item -ItemType Directory -Path $tempDir | Out-Null

Copy-Item "extension-firefox\manifest.json" -Destination $tempDir
Copy-Item "extension-firefox\background.js" -Destination $tempDir
Copy-Item "extension-firefox\content.js" -Destination $tempDir
Copy-Item "extension-firefox\content.css" -Destination $tempDir
Copy-Item "extension-firefox\popup.html" -Destination $tempDir
Copy-Item "extension-firefox\popup.js" -Destination $tempDir
Copy-Item "extension-firefox\icons" -Destination $tempDir -Recurse

$zipPath = Join-Path $env:TEMP "rustget_ff.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path (Join-Path $tempDir "*") -DestinationPath $zipPath -Force
Move-Item -Path $zipPath -Destination $xpiOutput -Force

Remove-Item $tempDir -Recurse -Force
Write-Host "XPI built successfully:"
Get-Item $xpiOutput | Select-Object Name, Length
