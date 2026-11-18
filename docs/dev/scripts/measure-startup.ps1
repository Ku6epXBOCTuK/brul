param(
    [string]$Binary = ((cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory + '\release\hello.exe'),
    [ValidateRange(1, 100)][int]$Runs = 5,
    [ValidateRange(1, 120)][int]$TimeoutSeconds = 10
)

$ErrorActionPreference = 'Stop'

$log = Join-Path $env:USERPROFILE '.config\brul\hello.log'
$pattern = 'brul startup: first frame in (?<ms>[0-9]+(?:\.[0-9]+)?) ms, backend (?<backend>[a-z]+)'
$env:BRUL_STARTUP_LOG = '1'

function Get-StartupLine {
    if (-not (Test-Path $log)) {
        return $null
    }
    return Get-Content $log | Where-Object { $_ -match $pattern } | Select-Object -Last 1
}

function Get-Median([double[]]$Values) {
    $sorted = @($Values | Sort-Object)
    $middle = [int]($sorted.Count / 2)
    if ($sorted.Count % 2 -eq 0) {
        return ($sorted[$middle - 1] + $sorted[$middle]) / 2
    }
    return $sorted[$middle]
}

foreach ($backend in @('software', 'gpu')) {
    $env:BRUL_EXAMPLE_BACKEND = $backend
    $samples = @()

    for ($run = 0; $run -lt $Runs; $run++) {
        Remove-Item $log -ErrorAction SilentlyContinue
        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        $process = Start-Process -FilePath $Binary -PassThru
        $line = $null

        while ($timer.Elapsed.TotalSeconds -lt $TimeoutSeconds -and -not $process.HasExited) {
            $line = Get-StartupLine
            if ($line) {
                break
            }
            Start-Sleep -Milliseconds 20
        }
        $timer.Stop()

        $reported = [double]::NaN
        if ($line -and $line -match $pattern) {
            $reported = [double]::Parse($Matches['ms'], [cultureinfo]::InvariantCulture)
        }
        $samples += [pscustomobject]@{
            Reported = $reported
            External = $timer.Elapsed.TotalMilliseconds
        }

        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
            $process.WaitForExit()
        }
    }

    $found = @($samples.Reported | Where-Object { -not [double]::IsNaN($_) })
    if ($found.Count -eq 0) {
        Write-Output "backend $backend : no startup line in $Runs runs"
        continue
    }

    $reported = @($found | Sort-Object)
    $external = @($samples.External | Sort-Object)
    Write-Output ("backend {0} ({1}/{2} runs): reported ms min {3} median {4} max {5} | spawn-to-line ms median {6}" -f `
        $backend, $found.Count, $Runs, `
        $reported[0].ToString('F1', [cultureinfo]::InvariantCulture), `
        (Get-Median $reported).ToString('F1', [cultureinfo]::InvariantCulture), `
        $reported[-1].ToString('F1', [cultureinfo]::InvariantCulture), `
        (Get-Median $external).ToString('F1', [cultureinfo]::InvariantCulture))
}
