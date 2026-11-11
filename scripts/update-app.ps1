# Cosmo Downloader - In-Place Application Updater Shortcut
# Forwards all arguments to scripts/deploy_binaries.ps1
[CmdletBinding()]
param(
    [ValidateSet("Dev", "Release")]
    [string]$Mode = "Release",

    [string]$InstallPath = "",

    [string]$InstallerUrl = "",

    [string]$ExpectedSha256 = ""
)

$PSScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition
$DeployScript = Join-Path $PSScriptRoot "deploy_binaries.ps1"

& $DeployScript -Mode $Mode -InstallPath $InstallPath -InstallerUrl $InstallerUrl -ExpectedSha256 $ExpectedSha256

