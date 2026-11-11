# RustGet Release Build and Installer Packaging Script
param(
    [switch]$SkipInstaller = $false
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RootDir = Split-Path -Parent $ScriptDir

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "         RustGet Release & Installer Builder          " -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

# 1. Setup Environment Paths
$llvmPkg = Get-Item "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\MartinStorsjo.LLVM-MinGW.UCRT_*\*\bin" -ErrorAction SilentlyContinue
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if ($llvmPkg) {
    $llvmBin = $llvmPkg.FullName
    $env:PATH = "$cargoBin;$llvmBin;$env:PATH"
    Write-Host "[OK] Toolchain LLVM-MinGW found at $llvmBin" -ForegroundColor Green
} else {
    $env:PATH = "$cargoBin;$env:PATH"
}

# 2. Generate Icons if needed
$iconPath = Join-Path $ScriptDir "icon.ico"
$extIconPath = Join-Path $RootDir "extension\icons\icon128.png"
if ((-not (Test-Path $iconPath)) -or (-not (Test-Path $extIconPath))) {
    Write-Host "[*] Generating application and extension icons..." -ForegroundColor Yellow
    $genScript = Join-Path $RootDir "scripts\generate_icons.ps1"
    & powershell -ExecutionPolicy Bypass -File $genScript
}

# 3. Cargo Release Build
Write-Host "[*] Compiling RustGet workspace in release mode..." -ForegroundColor Yellow
Push-Location $RootDir
try {
    cargo build --release --workspace
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo release build failed with code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}
Write-Host "[OK] Release binaries compiled successfully!" -ForegroundColor Green

# 4. Locate Binaries
$TargetReleaseDir = Join-Path $RootDir "target\x86_64-pc-windows-gnullvm\release"
if (-not (Test-Path (Join-Path $TargetReleaseDir "rustget-gui.exe"))) {
    $TargetReleaseDir = Join-Path $RootDir "target\release"
}

$GuiBin = Join-Path $TargetReleaseDir "rustget-gui.exe"
$CliBin = Join-Path $TargetReleaseDir "rustget.exe"
$HostBin = Join-Path $TargetReleaseDir "rustget-host.exe"

foreach ($bin in @($GuiBin, $CliBin, $HostBin)) {
    if (-not (Test-Path $bin)) {
        throw "Required binary missing: $bin"
    }
    $sizeMb = [Math]::Round((Get-Item $bin).Length / 1MB, 2)
    $leaf = Split-Path -Leaf $bin
    Write-Host "  -> $leaf : $sizeMb MB" -ForegroundColor DarkGray
}

# 5. Create Staging & Portable Package
$DistDir = Join-Path $RootDir "installer\dist"
if (Test-Path $DistDir) {
    Remove-Item $DistDir -Recurse -Force
}
New-Item -ItemType Directory -Path $DistDir | Out-Null

$PortableDir = Join-Path $DistDir "RustGet-v2.0.0-windows-x64"
New-Item -ItemType Directory -Path $PortableDir | Out-Null
$ExtDir = Join-Path $PortableDir "extension"
New-Item -ItemType Directory -Path $ExtDir | Out-Null
$ExtFfDir = Join-Path $PortableDir "extension-firefox"
New-Item -ItemType Directory -Path $ExtFfDir | Out-Null

Copy-Item $GuiBin -Destination "$PortableDir\"
Copy-Item $CliBin -Destination "$PortableDir\"
Copy-Item $HostBin -Destination "$PortableDir\"
if (Test-Path (Join-Path $RootDir "yt-dlp.exe")) {
    Copy-Item (Join-Path $RootDir "yt-dlp.exe") -Destination "$PortableDir\"
}
Copy-Item (Join-Path $ScriptDir "icon.ico") -Destination "$PortableDir\"
Copy-Item (Join-Path $RootDir "LICENSE") -Destination "$PortableDir\"
Copy-Item (Join-Path $RootDir "README.md") -Destination "$PortableDir\"
Copy-Item (Join-Path $RootDir "extension\*") -Destination $ExtDir -Recurse
Copy-Item (Join-Path $RootDir "extension-firefox\*") -Destination $ExtFfDir -Recurse

# Compress Portable Zip
$ZipPath = Join-Path $DistDir "RustGet-v2.0.0-windows-x64.zip"
Write-Host "[*] Creating portable ZIP archive: $ZipPath" -ForegroundColor Yellow
Compress-Archive -Path (Join-Path $PortableDir "*") -DestinationPath $ZipPath -Force
Write-Host "[OK] Portable package ready!" -ForegroundColor Green

# 6. Build Inno Setup Installer
if (-not $SkipInstaller) {
    Write-Host "[*] Checking for Inno Setup compiler (ISCC.exe)..." -ForegroundColor Yellow
    $localAppData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
    $isccPaths = @(
        (Join-Path $localAppData "Programs\Inno Setup 6\ISCC.exe"),
        "C:\Users\steam\AppData\Local\Programs\Antigravity IDE\resources\app\node_modules\innosetup\bin\ISCC.exe",
        "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        "C:\Program Files\Inno Setup 6\ISCC.exe"
    )

    $iscc = $null
    foreach ($p in $isccPaths) {
        if (Test-Path $p) {
            $iscc = $p
            break
        }
    }

    if (-not $iscc) {
        $cmdCheck = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
        if ($cmdCheck) {
            $iscc = $cmdCheck.Source
        }
    }

    if ($iscc) {
        Write-Host "[*] Compiling installer with $iscc..." -ForegroundColor Yellow
        Push-Location $ScriptDir
        try {
            & $iscc "RustGet.iss"
            if ($LASTEXITCODE -eq 0) {
                $setupPath = Join-Path $DistDir "RustGet-Setup-2.0.0.exe"
                Write-Host "[OK] Windows Installer successfully built at: $setupPath" -ForegroundColor Green
            } else {
                Write-Warning "Inno Setup compilation exited with code $LASTEXITCODE"
            }
        } finally {
            Pop-Location
        }
    } else {
        Write-Host "------------------------------------------------------" -ForegroundColor Yellow
        Write-Host " Inno Setup (ISCC.exe) was not found on this system. " -ForegroundColor Yellow
        Write-Host " To compile the installer EXE, install Inno Setup 6: " -ForegroundColor Yellow
        Write-Host "   winget install JRSoftware.InnoSetup              " -ForegroundColor Cyan
        Write-Host " Then re-run this script:                            " -ForegroundColor Yellow
        Write-Host "   .\installer\build-release.ps1                     " -ForegroundColor Cyan
        Write-Host " Portable build is fully ready in installer\dist\    " -ForegroundColor Green
        Write-Host "------------------------------------------------------" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "All release artifacts available in: $DistDir" -ForegroundColor Cyan
