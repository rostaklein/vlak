param([switch]$Offline)

$ErrorActionPreference = 'Stop'
$vlakRoot = Split-Path -Parent $PSScriptRoot
$vlakDownloads = Join-Path $vlakRoot 'downloads'

Push-Location $vlakRoot
try {
    $vlakBuildArgs = @('build', '--release', '--locked')
    if ($Offline) { $vlakBuildArgs += '--offline' }
    & cargo @vlakBuildArgs
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed; public download was not updated.' }

    $vlakSource = Join-Path $vlakRoot 'target\release\vlak.exe'
    if (-not (Test-Path -LiteralPath $vlakSource -PathType Leaf)) {
        throw 'Windows executable not found. Run this script on Windows using the MSVC toolchain.'
    }

    New-Item -ItemType Directory -Path $vlakDownloads -Force | Out-Null
    $vlakDestination = Join-Path $vlakDownloads 'vlak.exe'
    Copy-Item -LiteralPath $vlakSource -Destination $vlakDestination -Force
    $vlakHash = (Get-FileHash -LiteralPath $vlakDestination -Algorithm SHA256).Hash.ToLowerInvariant()
    $vlakChecksum = Join-Path $vlakDownloads 'SHA256SUMS'
    [System.IO.File]::WriteAllText($vlakChecksum, "$vlakHash  vlak.exe`n", [System.Text.Encoding]::ASCII)

    Write-Host 'Ready to commit: downloads/vlak.exe and downloads/SHA256SUMS'
    Write-Host "SHA256: $vlakHash"
} finally {
    Pop-Location
}
