# Cleanup non-zip files and keep only the distribution ZIPs
$ErrorActionPreference = "Stop"

$distDir = "RustGet-v2.0-Distribution"
$stagingDir = "RustGet-Staging"

if (Test-Path $stagingDir) {
    Remove-Item $stagingDir -Recurse -Force
}
New-Item -ItemType Directory -Path $stagingDir | Out-Null

# 1. Stage the clean full package
Write-Host "[*] Staging files for complete release archive..."
Copy-Item (Join-Path $distDir "RustGet-Setup-2.0.0.exe") -Destination $stagingDir
Copy-Item (Join-Path $distDir "HOW_TO_INSTALL.txt") -Destination $stagingDir
Copy-Item (Join-Path $distDir "Portable") -Destination $stagingDir -Recurse
Copy-Item (Join-Path $distDir "Browser-Extensions") -Destination $stagingDir -Recurse

# 2. Create the all-in-one distribution ZIP
$cleanZip = Join-Path $distDir "RustGet-v2.0.zip"
if (Test-Path $cleanZip) {
    Remove-Item $cleanZip -Force
}
Write-Host "[*] Creating clean RustGet-v2.0.zip..."
Compress-Archive -Path (Join-Path $stagingDir "*") -DestinationPath $cleanZip -Force

# Also put a copy in the workspace root
Copy-Item $cleanZip -Destination "RustGet-v2.0.zip" -Force

# Clean staging folder
Remove-Item $stagingDir -Recurse -Force

# 3. Remove all non-zip files and folders from the distribution folder
Write-Host "[*] Removing all non-zip files from $distDir..."
Get-ChildItem -Path $distDir | Where-Object { $_.Extension -ne ".zip" } | Remove-Item -Recurse -Force

# 4. Remove old master zip from workspace root
if (Test-Path "RustGet-v2.0-All-In-One.zip") {
    Remove-Item "RustGet-v2.0-All-In-One.zip" -Force
}

Write-Host "[OK] Finished! Only zip files remain in $distDir."
Get-ChildItem -Path $distDir | Select-Object Name, Length
