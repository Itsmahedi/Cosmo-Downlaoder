$chromeJson = @'
{
  "name": "com.cosmodownloader.downloader",
  "description": "CosmoDownloader Native Messaging Host for Google Chrome",
  "path": "cosmodownloader-host.exe",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://*",
    "chrome-extension://knldjmfmopnplkmfocnjbfkbbdjodkdf/"
  ]
}
'@

$edgeJson = @'
{
  "name": "com.cosmodownloader.downloader",
  "description": "CosmoDownloader Native Messaging Host for Microsoft Edge",
  "path": "cosmodownloader-host.exe",
  "type": "stdio",
  "allowed_origins": [
    "extension://*",
    "chrome-extension://*"
  ]
}
'@

$firefoxJson = @'
{
  "name": "com.cosmodownloader.downloader",
  "description": "CosmoDownloader Native Messaging Host for Mozilla Firefox",
  "path": "cosmodownloader-host.exe",
  "type": "stdio",
  "allowed_extensions": [
    "cosmodownloader@downloader.addon",
    "rustget@downloader.addon"
  ]
}
'@

$legacyChromeJson = @'
{
  "name": "com.rustget.downloader",
  "description": "CosmoDownloader Native Messaging Host for Google Chrome",
  "path": "cosmodownloader-host.exe",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://*",
    "chrome-extension://knldjmfmopnplkmfocnjbfkbbdjodkdf/"
  ]
}
'@

$legacyEdgeJson = @'
{
  "name": "com.rustget.downloader",
  "description": "CosmoDownloader Native Messaging Host for Microsoft Edge",
  "path": "cosmodownloader-host.exe",
  "type": "stdio",
  "allowed_origins": [
    "extension://*",
    "chrome-extension://*"
  ]
}
'@

$legacyFirefoxJson = @'
{
  "name": "com.rustget.downloader",
  "description": "CosmoDownloader Native Messaging Host for Mozilla Firefox",
  "path": "cosmodownloader-host.exe",
  "type": "stdio",
  "allowed_extensions": [
    "cosmodownloader@downloader.addon",
    "rustget@downloader.addon"
  ]
}
'@

$targetDirs = @(
  "c:\Users\steam\OneDrive\Documents\Antigravity\CosmoDownloader-v3.0\Portable",
  "c:\Users\steam\OneDrive\Documents\Antigravity\CosmoDownloader-v3.0\Browser-Extensions\Mozilla-Firefox",
  "c:\Users\steam\OneDrive\Documents\Antigravity\CosmoDownloader-v3.0\Browser-Extensions\Chrome-Edge-Brave-Opera",
  "c:\Users\steam\AppData\Local\Programs\CosmoDownloader",
  "c:\Users\steam\AppData\Local\Programs\CosmoDownloader\Browser-Extensions\Mozilla-Firefox",
  "c:\Users\steam\AppData\Local\Programs\CosmoDownloader\Browser-Extensions\Chrome-Edge-Brave-Opera"
)

foreach ($dir in $targetDirs) {
  if (Test-Path $dir) {
    [System.IO.File]::WriteAllText((Join-Path $dir "com.cosmodownloader.downloader.chrome.json"), $chromeJson)
    [System.IO.File]::WriteAllText((Join-Path $dir "com.cosmodownloader.downloader.edge.json"), $edgeJson)
    [System.IO.File]::WriteAllText((Join-Path $dir "com.cosmodownloader.downloader.firefox.json"), $firefoxJson)
    [System.IO.File]::WriteAllText((Join-Path $dir "com.rustget.downloader.chrome.json"), $legacyChromeJson)
    [System.IO.File]::WriteAllText((Join-Path $dir "com.rustget.downloader.edge.json"), $legacyEdgeJson)
    [System.IO.File]::WriteAllText((Join-Path $dir "com.rustget.downloader.firefox.json"), $legacyFirefoxJson)
  }
}
Write-Host "Updated all Native Messaging JSON manifests across all directories."
