<#
.SYNOPSIS
    Synchronizes the 3-part semantic version across Cosmo Downloader's single source of truth (package.json)
    and all build/runtime manifests (Cargo.toml, Cargo.lock, browser extension manifests, and Inno Setup installer).

.EXAMPLE
    .\scripts\set-version.ps1 -Version 1.14.0
    .\scripts\set-version.ps1 1.14.0
    .\scripts\set-version.ps1   # Reads version from package.json and syncs all other files
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)

$PkgJsonPath = Join-Path $RootDir "package.json"
if (-not (Test-Path $PkgJsonPath)) {
    throw "Single source of truth not found: $PkgJsonPath"
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $pkg = Get-Content -Raw -Encoding UTF8 $PkgJsonPath | ConvertFrom-Json
    $Version = [string]$pkg.version
}

$CleanVersion = $Version.Trim().TrimStart('v', 'V')
if ($CleanVersion -notmatch '^\d+\.\d+\.\d+$') {
    throw "Invalid version '$Version'. Expected 3-part semantic version MAJOR.MINOR.PATCH (e.g. 1.13.0)."
}

function Write-Utf8NoBom {
    param([string]$Path, [string]$Text)
    [System.IO.File]::WriteAllText($Path, $Text, $Utf8NoBom)
}

Write-Host "Syncing Cosmo Downloader version -> $CleanVersion"

# 1. Single source of truth: package.json
$pkgRaw = Get-Content -Raw -Encoding UTF8 $PkgJsonPath
$pkgUpdated = [regex]::Replace(
    $pkgRaw,
    '(?m)^(\s*"version"\s*:\s*")[^"]*(")',
    "`${1}$CleanVersion`${2}",
    1
)
Write-Utf8NoBom -Path $PkgJsonPath -Text $pkgUpdated
Write-Host "  [ok] package.json"

# 2. Cargo workspace version: Cargo.toml
$CargoTomlPath = Join-Path $RootDir "Cargo.toml"
$cargoRaw = Get-Content -Raw -Encoding UTF8 $CargoTomlPath
$cargoUpdated = [regex]::Replace(
    $cargoRaw,
    '(?ms)(\[workspace\.package\][^\[]*?^version\s*=\s*")[^"]*(")',
    "`${1}$CleanVersion`${2}",
    1
)
Write-Utf8NoBom -Path $CargoTomlPath -Text $cargoUpdated
Write-Host "  [ok] Cargo.toml"

# 3. Cargo.lock (keep workspace crate versions consistent without network access)
$CargoLockPath = Join-Path $RootDir "Cargo.lock"
if (Test-Path $CargoLockPath) {
    $lockRaw = Get-Content -Raw -Encoding UTF8 $CargoLockPath
    $workspaceCrates = @("cosmodownloader-cli", "cosmodownloader-native-host", "rustget-app", "rustget-core")
    foreach ($crate in $workspaceCrates) {
        $escaped = [regex]::Escape($crate)
        $lockRaw = [regex]::Replace(
            $lockRaw,
            "(?m)(name = `"$escaped`"\r?\nversion = `")[^`"]*(`")",
            "`${1}$CleanVersion`${2}"
        )
    }
    Write-Utf8NoBom -Path $CargoLockPath -Text $lockRaw
    Write-Host "  [ok] Cargo.lock"
}

# 4. Browser extension manifests & shared assets
$ManifestPaths = @(
    (Join-Path $RootDir "browser-extensions\Chrome-Edge-Brave-Opera\manifest.json"),
    (Join-Path $RootDir "browser-extensions\Mozilla-Firefox\manifest.json")
)
foreach ($mp in $ManifestPaths) {
    if (Test-Path $mp) {
        $mRaw = Get-Content -Raw -Encoding UTF8 $mp
        $mRaw = [regex]::Replace($mRaw, '(?m)^(\s*"version"\s*:\s*")[^"]*(")', "`${1}$CleanVersion`${2}", 1)
        $mRaw = [regex]::Replace($mRaw, '(?m)^(\s*"version_name"\s*:\s*")[^"]*(")', "`${1}v$CleanVersion`${2}", 1)
        Write-Utf8NoBom -Path $mp -Text $mRaw
        Write-Host "  [ok] $($mp.Replace($RootDir + [IO.Path]::DirectorySeparatorChar, ''))"
    }
}

$ChromeDir = Join-Path $RootDir "browser-extensions\Chrome-Edge-Brave-Opera"
$FirefoxDir = Join-Path $RootDir "browser-extensions\Mozilla-Firefox"
foreach ($sharedFile in @("content.css", "content.js", "background.js", "popup.css", "popup.html", "popup.js")) {
    $src = Join-Path $ChromeDir $sharedFile
    $dst = Join-Path $FirefoxDir $sharedFile
    if (Test-Path $src) {
        Copy-Item -Path $src -Destination $dst -Force
    }
}
Write-Host "  [ok] Synced shared extension files to Mozilla-Firefox"

# 5. Inno Setup installer script: installer/cosmodownloader.iss
$IssPath = Join-Path $RootDir "installer\cosmodownloader.iss"
if (Test-Path $IssPath) {
    $issRaw = Get-Content -Raw -Encoding UTF8 $IssPath
    $issUpdated = [regex]::Replace(
        $issRaw,
        '(?m)^(#define\s+MyAppVersion\s+")[^"]*(")',
        "`${1}$CleanVersion`${2}",
        1
    )
    Write-Utf8NoBom -Path $IssPath -Text $issUpdated
    Write-Host "  [ok] installer\cosmodownloader.iss"
}

Write-Host "Done. All project files are synced to v$CleanVersion."
