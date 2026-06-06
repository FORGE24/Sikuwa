# RFC: PyStat / DTSS（动态类型静态化）

| 字段 | 值 |
|------|-----|
| 状态 | Planned (Plan 2) |
| 依赖 | PythonIR v1 |

## 摘要

在 PythonIR 之上附加类型信息，将 Python 动态语义映射到静态 slot（S0–S3），驱动 Sikuwa-C codegen。

## Slot 等级

| Slot | 名称 | Codegen |
|------|------|---------|
| S0 | static | 原生 C 值 / struct |
| S1 | tagged | tagged union |
| S2 | boxed | 容器 + dyn 元素 |
| S3 | dyn | `sikuwa_value_t` / Py 桥 |

## 分析 Pass

1. Pass0 — 解析（复用 PIR）
2. Pass1 — 注解采集（PEP 484、`.pyi`）
3. Pass2 — 局部推断
4. Pass3 — 流敏感 narrow
5. Pass4 — 约束求解
6. Pass5 — 降级决策

## 配置

```toml
[sikuwa.pystat]
enabled = true
mode = "progressive"   # strict | progressive | compat
min_slot = "tagged"
allow_dyn_fallback = true
```

## 诊断码（预留）

- `SKW-T001` — 注解冲突
- `SKW-T002` — strict 无法静态化
- `SKW-T003` — 跨模块 ABI 变更
- `SKW-T004` — 动态特性强制 dyn
- `SKW-T005` — Profile 不一致

## Plan 2 交付

- `sikuwa-pystat` crate
- `sikuwa pystat report`
- S0 codegen：`int` / `str` 函数

## 参考

- [a2-architecture.md](./a2-architecture.md)
- [pythonir-v1.md](./pythonir-v1.md)
