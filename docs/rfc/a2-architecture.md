# RFC: Sikuwa 2.0 Ver.A2 总体架构

| 字段 | 值 |
|------|-----|
| 状态 | Accepted (Plan 1) |
| 版本 | 2.0.0-alpha.1 |
| 代号 | Sikuwa 2026/6/6 Ver.A2 |

## 摘要

Sikuwa 2.0 使用 **Rust** 作为主控层，**C** 提供稳定 ABI 与热路径，**x86 ASM** 加速 hash/算术热点。编译管线：

```text
Python 源码 → PythonIR (.pirb) → PyStat (.pstat) → Sikuwa-C → 原生产物
```

Nuitka 保留为可选 **Backend**（外部进程），不纳入自研 IR。

## Crate 划分

| Crate | 职责 |
|-------|------|
| `sikuwa-cli` | 命令行入口 |
| `sikuwa-core` | 版本、错误、公共类型 |
| `sikuwa-config` | TOML schema v2 |
| `sikuwa-pir` | PythonIR |
| `sikuwa-pystat` | DTSS（Plan 2+） |
| `sikuwa-engine` | BuildEngine（Plan 2+） |
| `sikuwa-codegen-c` | Codegen（Plan 3+） |

## C / ASM 层（Plan 3+）

- `c/sikuwa_cache/` — mmap LRU（移植 1.x cpp_cache）
- `c/sikuwa_pir_graph/` — 符号依赖图
- `asm/x86_64/` — SIMD hash、整数运算

## 缓存目录

统一为 `.sikuwa/`：

```text
.sikuwa/pir/
.sikuwa/pystat/
.sikuwa/build/
.sikuwa/logs/
```

## 里程碑

| 阶段 | 交付 |
|------|------|
| Plan 1 | Workspace + PIR 骨架 + CI |
| Plan 2 | Lowering + PyStat + Nuitka |
| Plan 3 | Lowering v1.2 + PyStat + Sikuwa-C codegen |
| Plan 4 | C ABI + manifest + link + S3 runtime ✅ |
| Plan 5 | @c_extern + struct codegen + py shim + manifest imports ✅ |

## 参考

- [pythonir-v1.md](./pythonir-v1.md)
- [dtss-pystat.md](./dtss-pystat.md)
- [../PLAN1.md](../PLAN1.md)
