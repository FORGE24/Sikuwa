# Plan 2 — PythonIR Lowering

**状态**：v1.1 持续完善中  
**依赖**：Plan 1（PythonIR 骨架）

## 已完成

### v1.0（Plan 2 初版）

- [x] `rustpython-parser` / `rustpython-ast` 接入
- [x] AST → PIR Lowering（`def`、return、if/else、算术、比较、一元运算）
- [x] `sikuwa pir build` / `text` / `verify` / `dump`

### v1.1（PythonIR 完善）

- [x] **SSA Phi 节点** — 控制流 merge 点自动插入（为 PGTE / 原位类型替换打基础）
- [x] **局部变量环境** — `load_fast` / `store_fast` + `locals` 列表（LogicalSlot 键）
- [x] **赋值** — `=`、`+=` 等 augassign
- [x] **循环** — `while`、`for x in iter`
- [x] **调用** — 直接名称调用 `call`
- [x] **模块 exports** — 导出符号表
- [x] **verify 增强** — phi / 块 ID / exports 校验
- [x] fixtures：`sum_range.py`、`total.py`

## 支持子集

| 语法 | 状态 |
|------|------|
| 顶层 `def` | ✓ |
| `return` / `if` / `elif` / `else` | ✓ |
| `=` / `+=` 等 augassign | ✓ |
| `while` / `for ... in ...` | ✓ |
| 直接函数调用 `foo(a, b)` | ✓ |
| 算术 / 比较 / 一元运算 | ✓ |
| Phi @ merge | ✓ |
| `class` / 闭包 / 下标 / 属性 | Plan 3 |
| `async def` | 报错 |

## 命令

```bash
cargo run -- pir build tests/fixtures/sum_range.py
cargo run -- pir text tests/fixtures/clamp.py
cargo run -- pir dump add.pirb
```

## Plan 3 预览

- `class` 基础、闭包
- PGTE / PyStat（隐形静态化）
- Sikuwa-C codegen
