# RFC: PIR Keyword Optimization Passes

| 字段 | 值 |
|------|-----|
| 状态 | Active (Plan 6 起步) |
| Pass 数量 | **35**（Python 3.11 关键词一一对应） |
| 依赖 | PythonIR v1.1+ |

## 摘要

PIR 中端优化在 **Lowering 之后**运行，**不依赖 Python AST**。每个 Python 3.11 保留关键词对应一个 LLVM 风格的优化 Pass，由 `sikuwa-pir/src/opt/` 实现。

```text
.py → AST → PIR (lower) → [35 keyword passes] → PyStat → Codegen
```

## Pass 管线

| 级别 | 说明 |
|------|------|
| **O0** | 不运行优化 |
| **O1** | 常量折叠、CFG 化简、Phi 化简、DCE（17 个核心 pass） |
| **O2** | O1 + 全 35 pass（含 inline 分析、import DCE 等 scaffold） |

## 已实现（O1）

| 关键词 | LLVM 类比 | 行为 |
|--------|-----------|------|
| `False`/`None`/`True` | constant-folding | 字面量传播、`is` 折叠 |
| `not` | instcombine | 常量 `not`、双重否定消除 |
| `if`/`elif`/`else` | simplifycfg | 常量条件分支折叠、不可达块删除 |
| `while`/`for` | loop-simplify | 复用 CFG 化简 |
| `return` | simplifycfg | 合并相同 return 块 |
| `del` | dce | 无侧效应且零 use 的 SSA 删除 |
| `is`/`in` | instcombine | 常量比较折叠 |

## 已实现（O2 新增）

| 关键词 | LLVM 类比 | 行为 |
|--------|-----------|------|
| `def` | inline | 同模块单块函数内联（如 `add` → `twice`） |
| `import`/`from` | globaldce | 移除未使用的 import 元数据 |
| `try`/`except`/`finally` | simplifycfg (exception) | 不可 raise 的 try 块删除 handler |

`codegen c --opt` 默认 **O2**，在 PyStat 前运行完整 PIR 优化。

## 已实现（O2）

| 关键词 | LLVM 类比 | 行为 |
|--------|-----------|------|
| `def` | inline | 同模块单块小函数内联（`pass_def_inline`） |
| `import`/`from` | globaldce | 删除未被 `call` 引用的 `ModuleImport` |
| `try`/`except`/`finally` | simplifycfg (exception) | 裁剪 protected 区不可能抛出的 handler 块 |
| `as` | instcombine (binding) | 块内 `store_fast`→`load_fast` 拷贝传播 |

## 脚手架（O2，待完善）

`async`/`await`、`with`/`yield`、`class` 化简、`global`/`nonlocal` 提升、`assert`/`raise` 路径裁剪等已注册元数据，实现为空操作或分析占位。

## CLI

```bash
cargo run -- pir opt tests/fixtures/opt_const.py --text
cargo run -- pir opt --list-passes
cargo run -- pir build tests/fixtures/add.py --opt
```

## 参考实现

- `crates/sikuwa-pir/src/opt/`
- 测试：`opt/passes.rs`、`opt/mod.rs`
