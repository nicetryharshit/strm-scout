param([switch]$NoPause)

$ErrorActionPreference = "Stop"
$projectRoot = $PSScriptRoot
$logPath = Join-Path $projectRoot "build.log"

function Stop-Build([string]$Message) {
    Write-Host "`nBUILD FAILED" -ForegroundColor Red
    Write-Host $Message -ForegroundColor Red
    Write-Host "`nFull output: $logPath"
    if (-not $NoPause) { Read-Host "Press Enter to close" }
    exit 1
}

try {
    Set-Location $projectRoot
    Start-Transcript -Path $logPath -Force | Out-Null

    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if ((Test-Path $cargoBin) -and ($env:Path -notlike "*$cargoBin*")) {
        $env:Path = "$cargoBin;$env:Path"
    }

    foreach ($command in @("node.exe", "npm.cmd", "cargo.exe")) {
        if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
            throw "$command was not found. Install Node.js LTS and Rust stable, then reopen PowerShell. Rust must be available at $cargoBin."
        }
    }

    Write-Host "Installing JavaScript dependencies..."
    & npm.cmd install
    if ($LASTEXITCODE -ne 0) { throw "npm install failed with exit code $LASTEXITCODE." }

    Write-Host "Building the portable Windows executable..."
    & npm.cmd run tauri build
    if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE." }

    $exe = Join-Path $projectRoot "src-tauri\target\release\strm-inspector.exe"
    if (-not (Test-Path $exe)) { throw "Tauri completed without creating $exe." }

    $portable = Join-Path $projectRoot "dist\STRM-Inspector-Portable"
    New-Item -ItemType Directory -Force -Path $portable | Out-Null
    Copy-Item $exe (Join-Path $portable "STRM-Inspector.exe") -Force
    Compress-Archive -Path (Join-Path $portable "*") -DestinationPath (Join-Path $projectRoot "dist\STRM-Inspector-Portable.zip") -Force

    Write-Host "`nBUILD COMPLETE" -ForegroundColor Green
    Write-Host "Portable app: $portable"
    Write-Host "Shareable ZIP: $projectRoot\dist\STRM-Inspector-Portable.zip"
}
catch {
    Stop-Build $_.Exception.Message
}
finally {
    if (Get-Command Stop-Transcript -ErrorAction SilentlyContinue) {
        try { Stop-Transcript | Out-Null } catch {}
    }
}

if (-not $NoPause) { Read-Host "Press Enter to close" }
