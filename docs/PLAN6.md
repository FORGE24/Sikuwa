# Plan 6 — HPGI / PIR Opt 协同 → GA

**GA 锚点**：2026-06-06（Ver.A2）

## 阶段一：Plan 6a — HPGI 核心格代数 ✅

- [x] `LogicalType` + `normalize_union(K=4)`
- [x] `join` / `meet` 单元测试
- [x] `SparseEnvironment`
- [x] `PhysicalType::merge` → HPGI join

## 阶段二：Plan 6b — SCC + TaggedLayout ✅

- [x] Call Graph + Tarjan SCC + `MAX_SCC_ITER=8`
- [x] `materialize_slot` / `TaggedLayout`
- [x] manifest `tagged_slots[]`

## 阶段三：Plan 6c — 黄金管线 ✅

```text
PIR Lower → PIR Opt (O1) → HPGI → PIR Opt (O2) → Codegen C
```

- [x] `sikuwa-codegen-c/src/pipeline.rs` — `run_golden_pipeline`
- [x] `codegen c --opt` 默认走黄金管线
- [x] `codegen c --opt --single-pass` 单遍 O2（旧行为）
- [x] `pir build --opt` / `pir opt --pipeline`

### CLI

```bash
cargo run -- codegen c tests/fixtures/opt_const.py --out-dir out/ --opt
cargo run -- codegen c tests/fixtures/add.py --out-dir out/ --opt --single-pass
cargo run -- pir opt tests/fixtures/opt_const.py --pipeline --text
cargo run -- pir build tests/fixtures/add.py --opt
```

## 阶段四：Plan 7 — 事实注入与 ABI 守卫 → [PLAN7.md](PLAN7.md) ✅

- [x] Pass1：`# skw @type` / PEP 484 / `.pyi`
- [x] `@c_extern` 参数类型
- [x] `SKW-T003` + `pystat verify` + CI preset

## 阶段五：Plan 8 — Ver.A2 GA → [PLAN8.md](PLAN8.md)

- [ ] PyStat Pass2–Pass5 + T002/T004/T005
- [ ] S1/S3 codegen + 闭包/类 runtime 闭环
- [ ] 多模块 link + `sikuwa build` + GA 封板

## 参考

- [rfc/dtss-pystat.md](rfc/dtss-pystat.md)
- [rfc/pir-opt-keywords.md](rfc/pir-opt-keywords.md)
