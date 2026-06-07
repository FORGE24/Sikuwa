# PyStat CI preset — verify fixtures against golden manifests.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
cargo run --release --quiet -- pystat verify --preset ci --all
