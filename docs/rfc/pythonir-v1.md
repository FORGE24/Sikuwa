# RFC: PythonIR v1

| 字段 | 值 |
|------|-----|
| 状态 | Active (v1.1) |
| IR 版本 | `PIR_VERSION = 1` |
| Magic | `SIPIR\x01` |

## 核心类型

- `ValueId` — SSA 虚拟寄存器 `%n`
- `BlockId` — 基本块 `^name`
- `SymbolRef` — 符号 `@module.func`
- `Phi` — merge 点 SSA phi（含 Python 局部名 → **LogicalSlot**）
- `FuncDef.locals` — 函数内全部局部名（PGTE 输入）
- `Module.exports` — 模块导出符号

## 指令集（v1.1 节选）

| 族 | OpCode |
|----|--------|
| 算术 | `binop_*`, `unary_*` |
| 比较 | `compare_*` |
| 局部 | `load_fast`, `store_fast`, `phi` |
| 循环 | `get_iter`, `for_iter_next` |
| 调用 | `call`, `call_builtin` |
| 常量 | `const`, `build_list`, `build_tuple` |

## Phi 示例

```pir
^merge_3:
  %8 = phi x ([^then_1] %3, [^else_2] %4)
  ret %8
```

## Plan 2 交付

- AST → PIR Lowering（`crates/sikuwa-pir/src/lower/`）
- `sikuwa pir build` / `text` / `verify` / `dump`
- 见 [../PLAN2.md](../PLAN2.md)

## 参考实现

- `crates/sikuwa-pir/`
- 样例模块：`sample_add_module()`
