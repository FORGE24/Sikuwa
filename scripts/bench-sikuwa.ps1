# Sikuwa 2.0 — functional smoke + compile/runtime benchmark (Windows/MinGW).
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

function Measure-Step([string]$Label, [scriptblock]$Block) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $oldEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & $Block 2>&1 | Out-Null
    $exit = $LASTEXITCODE
    $ErrorActionPreference = $oldEap
    $sw.Stop()
    $ms = $sw.Elapsed.TotalMilliseconds
    Write-Host ("  {0,-42} {1,8:N0} ms" -f $Label, $ms)
    if ($exit -ne 0) { throw "step failed ($exit): $Label" }
    return $ms
}

$Gcc = if ($env:CC) { $env:CC } else { "gcc" }
$Build = Join-Path $Root "target/sikuwa-bench"
$Out = Join-Path $Build "out"
if (Test-Path $Build) { Remove-Item -Recurse -Force $Build }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

Write-Host ""
Write-Host "=== Sikuwa benchmark ($(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')) ==="
Write-Host ""

Write-Host "[1/4] Release toolchain"
$null = Measure-Step "cargo build --release" { cargo build --release --quiet 2>&1 | Out-Null }
$ErrorActionPreference = "Continue"
$ver = (cargo run --release --quiet -- version 2>&1 | Out-String).Trim()
$ErrorActionPreference = "Stop"
Write-Host "  $ver"

Write-Host ""
Write-Host "[2/4] CLI compile-time (single fixture, cold out-dir)"
$fixtures = @(
    @{ Name = "add.py";         Args = @("codegen", "c", "tests/fixtures/add.py", "--out-dir", "$Out/add") },
    @{ Name = "add.py --opt";   Args = @("codegen", "c", "tests/fixtures/add.py", "--out-dir", "$Out/add-opt", "--opt") },
    @{ Name = "sum_range --opt"; Args = @("codegen", "c", "tests/fixtures/sum_range.py", "--out-dir", "$Out/sum_range", "--opt") },
    @{ Name = "clamp --opt";    Args = @("codegen", "c", "tests/fixtures/clamp.py", "--out-dir", "$Out/clamp", "--opt") },
    @{ Name = "plan3 --opt";    Args = @("codegen", "c", "tests/fixtures/plan3.py", "--out-dir", "$Out/plan3", "--opt") },
    @{ Name = "pystat report add"; Args = @("pystat", "report", "tests/fixtures/add.py") },
    @{ Name = "pystat verify ci";  Args = @("pystat", "verify", "--preset", "ci", "--all") },
    @{ Name = "build plan5_caller"; Args = @("build", "tests/fixtures/plan5_caller.py", "-o", "$Out/build-dist", "--opt") }
)
$compileMs = @()
foreach ($f in $fixtures) {
    $ms = Measure-Step $f.Name { cargo run --release --quiet -- @($f.Args) 2>&1 | Out-Null }
    $compileMs += $ms
}

Write-Host ""
Write-Host "[3/4] End-to-end smoke (correctness)"
$smokes = @(
    @{ Name = "closure-smoke";    Script = "scripts/closure-smoke.ps1" },
    @{ Name = "multimodule-smoke"; Script = "scripts/multimodule-smoke.ps1" }
)
foreach ($s in $smokes) {
    $path = Join-Path $Root $s.Script
    if (-not (Test-Path $path)) {
        $sh = $path -replace '\.ps1$', '.sh'
        if (Test-Path $sh) {
            Write-Host "  $($s.Name): skip (bash only on this host)"
            continue
        }
    }
    if (Test-Path $path) {
        try {
            $oldEap = $ErrorActionPreference
            $ErrorActionPreference = "Continue"
            Measure-Step $s.Name { & powershell -ExecutionPolicy Bypass -File $path 2>&1 | Out-Null }
            $ErrorActionPreference = $oldEap
            Write-Host "  $($s.Name): ok"
        } catch {
            Write-Host "  $($s.Name): FAILED - $_"
        }
    }
}

Write-Host ""
Write-Host "[4/4] Native runtime microbench"
$AddDir = Join-Path $Out "add-opt"
$SumDir = Join-Path $Out "sum_range"
$DllAdd = Join-Path $Build "libadd.dll"
$DllSum = Join-Path $Build "libsum_range.dll"

Measure-Step "link libadd" {
    cargo run --release --quiet -- link shared $AddDir -o $DllAdd 2>&1 | Out-Null
}
Measure-Step "link libsum_range" {
    cargo run --release --quiet -- link shared $SumDir -o $DllSum 2>&1 | Out-Null
}

$BenchExe = Join-Path $Build "bench.exe"
& $Gcc -O2 -Wall -Werror "-I$Root/c/include" "-I$AddDir" "-I$SumDir" `
    -DSKW_BENCH_SUM_RANGE "$Root/tests/ffi/bench_harness.c" $DllAdd $DllSum `
    -o $BenchExe
& $BenchExe

Write-Host ""
$avg = if ($compileMs.Count -gt 0) { ($compileMs | Measure-Object -Average).Average } else { 0 }
Write-Host ("Summary: {0} CLI steps, avg {1:N0} ms, artifacts under {2}" -f $compileMs.Count, $avg, $Build)
Write-Host "Done."
