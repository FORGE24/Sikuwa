# Plan 3 — class / 闭包 / PGTE·ITR / Sikuwa-C

| 状态 | Ver.A2 Plan 3 ✅ |
|------|------------------|
| 依赖 | PythonIR v1.1 |

## 交付项

### PIR v1.2（Lowering 扩展）

- **属性**：`load_attr` / `store_attr`
- **下标**：`subscript_load` / `subscript_store`
- **闭包**：`load_cell` / `store_cell` / `make_closure`，`FuncDef.cellvars` + `nested`
- **类**：`ClassDef` + 方法 lowering（`module.classes`）
- Fixtures：`tests/fixtures/attrs.py`、`plan3.py`

### sikuwa-pystat

- **PGTE**：遍历 `locals` + `phi` + op 推断 `PhysicalType`
- **ITR**：64-bit 兼容类型（`int64` ↔ `bool`）同槽 `SlotStrategy::Itr`
- **输出**：`.pstat`（magic `SKPST\x01` + JSON）、CLI `sikuwa pystat report`

### sikuwa-codegen-c

- **S0 静态函数**：PyStat `static_eligible` 时 emit `int64_t` C 函数
- CLI：`sikuwa codegen c <file.py> --out-dir <dir>`

## 用法

```bash
# PIR
cargo run -- pir build tests/fixtures/plan3.py
cargo run -- pir text tests/fixtures/attrs.py

# PyStat
cargo run -- pystat report tests/fixtures/add.py --json
cargo run -- pystat report tests/fixtures/clamp.py -o add.pstat

# Sikuwa-C
cargo run -- codegen c tests/fixtures/add.py --out-dir out/
```

## 管线

```text
.py → PIR (.pirb) → PyStat (.pstat) → Sikuwa-C (.h/.c)
         ↑                    ↑
    class/闭包/属性/下标    PGTE + ITR slot 规划
```

## 后续（Plan 4+）

- `BuildClass` 模块级 emit、LFS L1/L2 回退
- 跨模块 PGTE 图、`.pyi` Pass1
- CFG 多基本块 C codegen（clamp 等）
