param(
    [string]$Exe,
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

$classes = "HKCU:\Software\Classes\bropicker"
$caps = "HKCU:\Software\bropicker-Capabilities"
$regApps = "HKCU:\Software\RegisteredApplications"
$install = Join-Path $env:LOCALAPPDATA "bropicker-brul"

if ($Uninstall) {
    Remove-Item -Recurse -Force $classes -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $caps -ErrorAction SilentlyContinue
    Remove-ItemProperty -Path $regApps -Name "bropicker" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $install -ErrorAction SilentlyContinue
    Write-Host "bropicker unregistered"
    exit 0
}

if (-not $Exe) {
    $Exe = Join-Path $PSScriptRoot "..\..\..\target\release\bropicker.exe"
}

if (-not (Test-Path -LiteralPath $Exe)) {
    Write-Error "bropicker.exe not found at $Exe. Build first: cargo build --release -p bropicker"
    exit 1
}

$assets = Join-Path $PSScriptRoot "..\assets"
if (-not (Test-Path -LiteralPath $assets)) {
    Write-Error "bropicker assets not found at $assets"
    exit 1
}

$source = (Resolve-Path -LiteralPath $Exe).Path
$target = Join-Path $install "bropicker.exe"
New-Item -ItemType Directory -Force -Path $install | Out-Null

if ($source -ne $target) {
    Copy-Item -LiteralPath $source -Destination $target -Force
}

$installedAssets = Join-Path $install "assets"
Remove-Item -Recurse -Force $installedAssets -ErrorAction SilentlyContinue
Copy-Item -LiteralPath $assets -Destination $install -Recurse -Force

$command = "`"$target`" `"%1`""

New-Item -Path "$classes\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path $classes -Name "(default)" -Value "bropicker"
Set-ItemProperty -Path $classes -Name "URL Protocol" -Value ""
Set-ItemProperty -Path "$classes\shell\open\command" -Name "(default)" -Value $command

New-Item -Path $caps -Force | Out-Null
Set-ItemProperty -Path $caps -Name "ApplicationName" -Value "bropicker"
Set-ItemProperty -Path $caps -Name "ApplicationDescription" -Value "Lightweight link picker: choose which browser opens the URL"
Set-ItemProperty -Path $caps -Name "ApplicationIcon" -Value "$target,0"

New-Item -Path "$caps\URLAssociations" -Force | Out-Null
Set-ItemProperty -Path "$caps\URLAssociations" -Name "https" -Value "bropicker"
Set-ItemProperty -Path "$caps\URLAssociations" -Name "http" -Value "bropicker"

New-Item -Path $regApps -Force | Out-Null
Set-ItemProperty -Path $regApps -Name "bropicker" -Value "Software\bropicker-Capabilities"

Write-Host "Installed: $install"
Write-Host "Registered: $target"
Write-Host "Select bropicker for HTTP/HTTPS in the opened Settings window."
Start-Process "ms-settings:defaultapps"
