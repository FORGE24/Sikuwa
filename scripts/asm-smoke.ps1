# x86_64 hotpath asm smoke — Windows MinGW (win-gnu GAS) + C reference parity.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Build = Join-Path $Root "target\asm-smoke"
New-Item -ItemType Directory -Force -Path $Build | Out-Null

function Resolve-MingwGcc {
    if ($env:CC -and (Test-Path $env:CC)) { return $env:CC }
    foreach ($name in @("gcc", "x86_64-w64-mingw32-gcc")) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd) { return $cmd.Source }
    }
    foreach ($path in @(
        "C:\msys64\mingw64\bin\gcc.exe",
        "C:\msys64\ucrt64\bin\gcc.exe",
        "C:\tools\msys64\mingw64\bin\gcc.exe"
    )) {
        if (Test-Path $path) { return $path }
    }
    throw "MinGW gcc not found — install MSYS2 (pacman -S mingw-w64-ucrt-x86_64-gcc) or set CC"
}

$Gcc = Resolve-MingwGcc
Write-Host "==> using MinGW gcc: $Gcc"

$Common = @(
    "-O2", "-Wall", "-Werror",
    "-DSKW_BUILDING_MODULE",
    "-I$Root\c\include"
)

Write-Host "==> compile asm hotpath test (win-gnu GAS)"
& $Gcc @Common "-DSKW_HOTPATH_ASM" `
    "$Root\c\src\hotpath\dispatch.c" `
    "$Root\c\src\runtime\value.c" `
    "$Root\asm\x86_64\win-gnu\hash.S" `
    "$Root\asm\x86_64\win-gnu\i64.S" `
    "$Root\asm\x86_64\win-gnu\tagged.S" `
    "$Root\tests\ffi\asm_hotpath_test.c" `
    "-o" (Join-Path $Build "asm_hotpath_test.exe")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$asmResult = & (Join-Path $Build "asm_hotpath_test.exe") 2>&1
if ($LASTEXITCODE -ne 0) { Write-Error $asmResult; exit $LASTEXITCODE }

Write-Host "==> C-only fallback (no asm symbols)"
& $Gcc @Common `
    "$Root\c\src\hotpath\dispatch.c" `
    "$Root\c\src\runtime\value.c" `
    "$Root\tests\ffi\asm_hotpath_test.c" `
    "-o" (Join-Path $Build "asm_hotpath_c_test.exe")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& (Join-Path $Build "asm_hotpath_c_test.exe")

Write-Host "asm smoke ok (MinGW)"
