# RFC: Python For LLVM IR（Sikuwa 2.5 / Ver.B1）

| 字段 | 值 |
|------|-----|
| 状态 | Draft（Plan 2.5 — 2027） |
| 依赖 | PythonIR v1、PyStat / DTSS、2.0 manifest ABI |
| 目标版本 | `2.5.0` |

## 摘要

在 2.0 **Sikuwa-C** 后端之外，新增 **Python → LLVM IR** 代码生成路径。前端（PIR lower、PyStat Pass0–5、黄金管线）与 2.0 共用；仅替换 emit 层为 LLVM IR，最终通过 `llc` / `clang` 链接为原生产物。

## 动机

| 维度 | Sikuwa-C（2.0） | LLVM IR（2.5） |
|------|-----------------|----------------|
| 优化 | C 编译器 LTO | LLVM 中端 / 后端 Pass 生态 |
| 跨平台 | 依赖 C ABI + 自研 runtime | LLVM 目标三元组统一 |
| 调试 | `.c` 可读 | `.ll` + LLVM 工具链 |
| 与 2.0 关系 | 已 GA | **并存 Backend**，非替换 |

## 管线

```text
Python 源码
    → PythonIR (.pirb)
    → PyStat (.pstat)          # 复用 DTSS / Slot S0–S3
    → [PIR Opt O1/O2]          # 复用黄金管线
    → LLVM IR emit (.ll / .bc)
    → llc / clang -shared
    → .so / .dll / .exe
```

## Crate 划分（规划）

| Crate | 职责 |
|-------|------|
| `sikuwa-codegen-llvm` | PIR + FuncStat → LLVM Module |
| `sikuwa-cli` | `codegen llvm`、`build --backend llvm` |
| `sikuwa-link` | 可选：LLVM 对象文件链接（或委托 clang） |

## Slot → LLVM 类型（草案）

| Slot | Logical / Physical | LLVM 表示 |
|------|-------------------|-----------|
| S0 | `int64` / `bool` | `i64` |
| S0 | `float64` | `double` |
| S0 | `str` | `i8*` 或 `{ i8*, i64 }`（与 manifest 一致） |
| S1 | tagged union | `%skw_tagged = type { i8, i64 }`（tag + payload） |
| S3 | dyn | `i8*` opaque + `@skw_*` runtime decl |

闭包 env、类 `self` 指针对应 `%struct.skw_env_*` / `%struct.skw_class_*`。

## Emit 范围（分阶段）

### 2.5a — S0 最小子集

- 单基本块 / 多基本块 CFG
- `Const`、`BinOp*`、`Compare*`、`Return`
- `LoadFast` / `StoreFast`
- 模块级函数 export 符号与 manifest 对齐

### 2.5b — S1 / S3 / 闭包

- `skw_tagged_t` 分支 unpack / pack
- S3：`LoadAttr`、`Call` 未知 callee → runtime stub
- `MakeClosure` / `LoadCell` / `StoreCell`

### 2.5c — 多模块与 build

- manifest `imports` → LLVM 外部符号 / 链接顺序
- `sikuwa build --backend llvm` 一体化
- `pystat verify` 扩展：C / LLVM tier 一致性（SKW-T005 泛化）

## CLI（规划）

```bash
sikuwa codegen llvm <file.py> --out-dir out/ [--opt]
sikuwa build <entry.py> -o dist/ --backend llvm [--opt]
sikuwa doctor                    # 检查 llvm-config / llc / clang
```

## 配置（规划）

```toml
[sikuwa.codegen]
backend = "llvm"   # c | llvm | nuitka

[sikuwa.codegen.llvm]
opt_level = 2      # 0–3，映射 LLVM Pass 级别
target_triple = "" # 空则 host default
```

## 非目标（2.5 GA 前）

- 完整 CPython C-API 兼容层
- JIT（ORC / MCJIT）— 可列为 2.6+
- 替代 Sikuwa-C 为默认 backend

## 风险

| 风险 | 缓解 |
|------|------|
| LLVM 版本碎片化 | CI pin 单一 LTS；`doctor` 检查 |
| S3 runtime 与 C backend 重复 | 共享 `c/` runtime 声明，LLVM 仅 emit `declare` + 调用 |
| manifest ABI 双 backend 漂移 | verify 预设 `ci-llvm`；golden `.ll` snapshot（可选） |

## 参考

- [../ROADMAP.md](../ROADMAP.md) — 2.5 里程碑
- [a2-architecture.md](a2-architecture.md) — 2.0 架构
- [native-c-ffi.md](native-c-ffi.md) — S0–S3 slot 与 ABI
- [dtss-pystat.md](dtss-pystat.md) — PyStat Pass 定义
