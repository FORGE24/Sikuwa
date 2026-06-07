# Plan 8c — closure + class smoke (plan3.py).
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root
$Build = Join-Path $Root "target/closure-smoke"
$Out = Join-Path $Build "plan3"
New-Item -ItemType Directory -Force -Path $Out | Out-Null

Write-Host "==> codegen plan3.py"
cargo run --quiet -- codegen c tests/fixtures/plan3.py --out-dir $Out

$So = Join-Path $Build "libplan3.dll"
Write-Host "==> link shared library"
cargo run --quiet -- link shared $Out -o $So

Write-Host "==> compile & run closure harness"
$Gcc = if ($env:CC) { $env:CC } else { "gcc" }
& $Gcc -O2 -Wall -Werror "-I$Root/c/include" "-I$Out" `
  "$Root/tests/ffi/closure_harness.c" $So -o "$Build/closure_harness.exe"
& "$Build/closure_harness.exe"
Write-Host "closure smoke ok"
