Stop-Process -Name "*rustget*" -Force -ErrorAction SilentlyContinue
Stop-Process -Name "*CosmoDownloader*" -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

$target = "C:\Users\steam\RustGet-Source\target\release"
$portable = "c:\Users\steam\OneDrive\Documents\Antigravity\CosmoDownloader-v3.0\Portable"
$installed = "C:\Users\steam\AppData\Local\Programs\CosmoDownloader"
$installedOld = "C:\Users\steam\AppData\Local\Programs\RustGet"

# 1. Update Portable directory
Copy-Item "$target\CosmoDownloader.exe" "$portable\CosmoDownloader.exe" -Force
Copy-Item "$target\cosmodownloader-cli.exe" "$portable\CosmoDownloader-CLI.exe" -Force
Copy-Item "$target\cosmodownloader-cli.exe" "$portable\cosmodownloader-cli.exe" -Force
Copy-Item "$target\cosmodownloader-host.exe" "$portable\CosmoDownloader-Host.exe" -Force
Copy-Item "$target\cosmodownloader-host.exe" "$portable\cosmodownloader-host.exe" -Force
Copy-Item "$target\rustget.exe" "$portable\rustget.exe" -Force
Copy-Item "$target\rustget-host.exe" "$portable\rustget-host.exe" -Force
Copy-Item "C:\Users\steam\RustGet-Source\crates\app\icon.ico" "$portable\icon.ico" -Force

# Remove any old rustget-host-real.exe from Portable
if (Test-Path "$portable\rustget-host-real.exe") {
    Remove-Item "$portable\rustget-host-real.exe" -Force
}

# 2. Update Installed directory
if (Test-Path $installed) {
    Copy-Item "$target\CosmoDownloader.exe" "$installed\CosmoDownloader.exe" -Force
    Copy-Item "$target\CosmoDownloader.exe" "$installed\rustget-gui.exe" -Force
    Copy-Item "$target\cosmodownloader-cli.exe" "$installed\CosmoDownloader-CLI.exe" -Force
    Copy-Item "$target\cosmodownloader-cli.exe" "$installed\cosmodownloader-cli.exe" -Force
    Copy-Item "$target\cosmodownloader-cli.exe" "$installed\rustget.exe" -Force
    Copy-Item "$target\cosmodownloader-host.exe" "$installed\CosmoDownloader-Host.exe" -Force
    Copy-Item "$target\cosmodownloader-host.exe" "$installed\cosmodownloader-host.exe" -Force
    Copy-Item "$target\rustget-host.exe" "$installed\rustget-host.exe" -Force
    Copy-Item "C:\Users\steam\RustGet-Source\crates\app\icon.ico" "$installed\icon.ico" -Force

    # Copy extension folders to installed directory
    $extSrc = "c:\Users\steam\OneDrive\Documents\Antigravity\CosmoDownloader-v3.0\Browser-Extensions"
    Copy-Item "$extSrc\Chrome-Edge-Brave-Opera\*" "$installed\Browser-Extensions\Chrome-Edge-Brave-Opera" -Recurse -Force
    Copy-Item "$extSrc\Mozilla-Firefox\*" "$installed\Browser-Extensions\Mozilla-Firefox" -Recurse -Force
    Write-Host "Updated installed Programs directory: $installed"
}

if (Test-Path $installedOld) {
    Copy-Item "$target\CosmoDownloader.exe" "$installedOld\CosmoDownloader.exe" -Force
    Copy-Item "$target\CosmoDownloader.exe" "$installedOld\rustget-gui.exe" -Force
    Copy-Item "$target\cosmodownloader-cli.exe" "$installedOld\rustget.exe" -Force
    Copy-Item "$target\cosmodownloader-host.exe" "$installedOld\rustget-host.exe" -Force
    Write-Host "Updated legacy Programs directory: $installedOld"
}

Write-Host "Deployed updated binaries and extensions!"
