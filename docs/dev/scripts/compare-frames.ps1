param(
    [string]$Binary = ((cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory + '\release\hello.exe'),
    [ValidateRange(1, 60)][int]$Seconds = 3,
    [ValidateRange(10, 120)][int]$TimeoutSeconds = 40
)

$ErrorActionPreference = 'Stop'
$out = Join-Path ((cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory) 'frame-compare'
New-Item -ItemType Directory -Force -Path $out | Out-Null

$startupPattern = 'brul startup: first frame in (?<first>[0-9]+(?:\.[0-9]+)?) ms, backend (?<backend>[a-z]+), scale (?<scale>[0-9.]+), surface (?<width>[0-9]+)x(?<height>[0-9]+)'
$rasterPattern = 'brul raster: backend (?<backend>[a-z]+), first (?<first>[0-9]+(?:\.[0-9]+)?) ms, steady median (?<median>[0-9]+(?:\.[0-9]+)?) ms, min (?<min>[0-9]+(?:\.[0-9]+)?) ms, max (?<max>[0-9]+(?:\.[0-9]+)?) ms over (?<frames>[0-9]+) frames'
$formatPattern = 'brul capture: surface format (?<format>[A-Za-z0-9]+)'

function Get-LogLine([string]$Path, [string]$Pattern) {
    if (-not (Test-Path $Path)) {
        return $null
    }
    return Get-Content $Path | Where-Object { $_ -match $Pattern } | Select-Object -Last 1
}

function Close-Process([System.Diagnostics.Process]$Process) {
    if ($Process.HasExited) {
        return
    }
    $null = $Process.CloseMainWindow()
    if (-not $Process.WaitForExit(5000)) {
        Stop-Process -Id $Process.Id -Force
        $Process.WaitForExit()
    }
}

$captures = @{}
foreach ($backend in @('software', 'gpu')) {
    $log = Join-Path $out "$backend.log"
    $dump = Join-Path $out "$backend.raw"
    Remove-Item $log, $dump -ErrorAction SilentlyContinue
    $env:BRUL_EXAMPLE_BACKEND = $backend
    $env:BRUL_STARTUP_LOG = '1'
    $env:BRUL_LOG_FILE = $log
    $env:BRUL_FRAME_DUMP = $dump

    $process = Start-Process -FilePath $Binary -PassThru
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    while ($timer.Elapsed.TotalSeconds -lt $TimeoutSeconds -and -not $process.HasExited) {
        $written = Test-Path $dump
        if ($written) {
            $age = (Get-Date) - (Get-Item $dump).LastWriteTime
            if ($age.TotalSeconds -ge $Seconds) {
                break
            }
        }
        Start-Sleep -Milliseconds 50
    }
    Close-Process $process

    $startup = Get-LogLine $log $startupPattern
    $raster = Get-LogLine $log $rasterPattern
    if (-not (Test-Path $dump)) {
        Write-Output "backend $backend : no frame dump in $TimeoutSeconds s"
        continue
    }
    $bytes = [System.IO.File]::ReadAllBytes($dump)
    $formatLine = Get-LogLine $log $formatPattern
    $format = if ($formatLine -and $formatLine -match $formatPattern) { $Matches['format'] } else { 'unknown' }
    if ($format.StartsWith('Bgra')) {
        for ($i = 0; $i -lt $bytes.Length; $i += 4) {
            $red = $bytes[$i]
            $bytes[$i] = $bytes[$i + 2]
            $bytes[$i + 2] = $red
        }
    }
    $captures[$backend] = $bytes
    $line = "backend $backend ($format) : $startup"
    if ($raster) {
        $line += " | $raster"
    } else {
        $line += ' | no raster line'
    }
    Write-Output $line
}

if ($captures.Count -ne 2) {
    return
}
$cpu = $captures['software']
$gpu = $captures['gpu']
if ($cpu.Length -ne $gpu.Length) {
    Write-Output "frame sizes differ: software $cpu bytes, gpu $gpu bytes"
    return
}

$diffPixels = 0
$maxChannel = 0
$sumChannel = 0
for ($i = 0; $i -lt $cpu.Length; $i += 4) {
    $pixelDiff = $false
    for ($channel = 0; $channel -lt 4; $channel++) {
        $diff = [Math]::Abs([int]$cpu[$i + $channel] - [int]$gpu[$i + $channel])
        if ($diff -gt 0) {
            $pixelDiff = $true
        }
        if ($diff -gt $maxChannel) {
            $maxChannel = $diff
        }
        $sumChannel += $diff
    }
    if ($pixelDiff) {
        $diffPixels++
    }
}
$mean = $sumChannel / $cpu.Length
Write-Output ("frames: {0} bytes, {1}/{2} pixels differ, max channel {3}, mean channel {4:F2}" -f `
    $cpu.Length, $diffPixels, ($cpu.Length / 4), $maxChannel, $mean)
