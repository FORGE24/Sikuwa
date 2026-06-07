# Plan 8d — multi-module build smoke (plan5_caller imports add).
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root
$Build = Join-Path $Root "target/multimodule-smoke"
$Out = Join-Path $Build "dist"
if (Test-Path $Build) { Remove-Item -Recurse -Force $Build }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

Write-Host "==> sikuwa build plan5_caller.py"
cargo run --quiet -- build tests/fixtures/plan5_caller.py -o $Out --opt

$So = Join-Path $Out "libplan5_caller.dll"
if (-not (Test-Path $So)) {
    throw "missing shared library: $So"
}

Write-Host "==> compile & run multimodule harness"
$Gcc = if ($env:CC) { $env:CC } else { "gcc" }
& $Gcc -O2 -Wall -Werror "-I$Root/c/include" `
  "-I$Out/add" "-I$Out/plan5_caller" `
  "$Root/tests/ffi/multimodule_harness.c" $So -o "$Out/harness.exe"
& "$Out/harness.exe"
Write-Host "multimodule smoke ok"
