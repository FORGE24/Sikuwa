# Changelog

## 2.0.0-beta.1 — 2026-06-07

Plan 8 (Ver.A2 GA beta) — PyStat Pass2–5、S0/S1/S3 codegen、闭包 runtime、多模块 `sikuwa build`。

### Added

- PyStat Pass2–Pass5：`strict`/`min_slot` 配置、流敏感 narrow（Pass3）、Call 实参约束（Pass4）、降级决策（Pass5）
- S1 `skw_tagged_t` / S3 `skw_value_t *` 分层 codegen
- 闭包 `MakeClosure` / `LoadCell` / `StoreCell` emit；`plan3.py` closure smoke
- 多模块拓扑 build：`sikuwa build entry.py -o dist/ --opt`（`plan5_caller` + `add`）
- CLI：`sikuwa build`；codegen/build 读取 `[sikuwa.pystat]`
- Smoke：`scripts/closure-smoke.*`、`scripts/multimodule-smoke.*`

### Changed

- `pystat verify` 增加 manifest slot 与 codegen tier 一致性检查（SKW-T005）
