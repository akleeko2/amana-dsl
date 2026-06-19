param(
    [Parameter(Mandatory = $true)]
    [string]$Source,
    [string]$Output = ".amana_e2e_dist",
    [int]$Port = 3210
)

$ErrorActionPreference = "Stop"

if (!(Test-Path -LiteralPath $Source)) {
    throw "Source file '$Source' does not exist. Pass a real .amana entry file, for example: scripts/e2e-smoke.ps1 -Source path\to\app.amana"
}

Write-Host "[Amana E2E] Building Amana CLI..."
cargo build
$Amana = Join-Path (Get-Location) "target/debug/amana.exe"

Write-Host "[Amana E2E] Checking source formatting..."
& $Amana fmt $Source --check

Write-Host "[Amana E2E] Checking Amana source with JSON diagnostics..."
& $Amana check $Source --json --snapshot-ir "$Output/source.ir.json"

Write-Host "[Amana E2E] Building generated app..."
& $Amana build $Source $Output

Write-Host "[Amana E2E] Installing Node.js dependencies..."
Push-Location $Output
npm install

Write-Host "[Amana E2E] Checking generated JavaScript syntax..."
node --check app.js
node --check runtime/engine.js
node --check middleware/security.js
node --check middleware/hooks-worker.js
Pop-Location

Write-Host "[Amana E2E] Starting generated server on port $Port..."
$job = Start-Job -ScriptBlock {
    param($WorkingDir, $ServerPort)
    Set-Location $WorkingDir
    $env:PORT = "$ServerPort"
    $env:SESSION_SECRET = "local_e2e_session_secret_change_me"
    npm start
} -ArgumentList (Resolve-Path $Output).Path, $Port

try {
    Start-Sleep -Seconds 5
    $response = Invoke-WebRequest -Uri "http://localhost:$Port/" -UseBasicParsing
    if ($response.StatusCode -ne 200) {
        throw "Expected HTTP 200, got $($response.StatusCode)"
    }
    Write-Host "[Amana E2E] HTTP smoke passed."
}
finally {
    Stop-Job $job -ErrorAction SilentlyContinue | Out-Null
    Remove-Job $job -Force -ErrorAction SilentlyContinue | Out-Null
}
