# Sikuwa 版本路线图

| 版本 | 目标日期 | 代号 / 锚点 | 状态 |
|------|----------|-------------|------|
| **2.0** | 2026 Q2 | Ver.A2 — Plan 8 GA | beta（2.0.0-beta.1） |
| **2.5** | 2027 | Ver.B1 — LLVM IR Backend | 规划中 |

---

## 2.0 — Ver.A2（2026）

PythonIR + PyStat Pass0–5 + Sikuwa-C（S0/S1/S3）+ 闭包 / 多模块 `sikuwa build`。

详见 [PLAN8.md](PLAN8.md)。

```text
.py → PythonIR (.pirb) → PyStat (.pstat) → Sikuwa-C → dll/so/exe
```

---

## 2.5 — Ver.B1（2027）

**目标版本**：`2.5.0`  
**GA 锚点**：2027 — 在 2.0 稳定 ABI 与 PyStat 管线之上，增加 **Python → LLVM IR** 原生后端。

### 核心目标

| # | 目标 | 说明 |
|---|------|------|
| B1 | **Python For LLVM IR** | 新增 `sikuwa-codegen-llvm`：PIR + PyStat 产物 → LLVM IR（`.ll` / `.bc`） |
| B2 | Slot 分层 → LLVM 类型映射 | S0 → `i64`/`double` 等；S1 → tagged struct；S3 → 不透明指针 + runtime 调用 |
| B3 | CLI 与 build 集成 | `sikuwa codegen llvm`、`sikuwa build --backend llvm` |
| B4 | 端到端 smoke | `add.py` / `clamp.py` 经 LLVM 管线生成可执行或 `.so` |
| B5 | 与 Sikuwa-C 并存 | 共享 PIR / PyStat / 黄金管线；manifest 标注 `codegen_backend` |

### 目标管线

```text
.py → PythonIR (.pirb) → PyStat (.pstat) → LLVM IR (.ll) → llc/clang → 原生产物
                              ↑
                    复用 2.0 HPGI / Pass0–5 / 黄金管线
```

### 阶段划分（草案）

| 阶段 | 焦点 | 出口 |
|------|------|------|
| 2.5a | `sikuwa-codegen-llvm` 骨架 + S0 emit（int/bool 函数） | `add.py` → `.ll` 可人工 inspect |
| 2.5b | S1 tagged + S3 dyn 桩；闭包 env struct → LLVM struct | `clamp.py` / `plan3.py` smoke |
| 2.5c | `sikuwa build --backend llvm`；与 C backend 同 manifest 校验 | G1–G5 等价 LLVM 矩阵 |
| 2.5d | 优化 Pass 对接（PIR O1/O2 后 LLVM opt 可选）；GA 封板 | `2.5.0` tag |

### 依赖与前置

- 2.0 GA：`2.0.0` soak 完成，PyStat / manifest ABI 稳定
- LLVM **15+**（或项目 pin 版本，见 `rust-toolchain` / CI 矩阵）
- 可选：`inkwell` 或手写 LLVM C API 绑定（实现阶段 RFC 定稿）

### 验收命令（目标）

```bash
cargo run -- codegen llvm tests/fixtures/add.py --out-dir out/ --opt
cargo run -- build tests/fixtures/add.py -o dist/ --backend llvm --opt
# llc / clang 链接后 C harness 调用通过
bash scripts/llvm-smoke.sh
```

### 参考

- [rfc/llvm-ir-backend.md](rfc/llvm-ir-backend.md) — LLVM 后端 RFC（草案）
- [rfc/a2-architecture.md](rfc/a2-architecture.md) — 2.0 总体架构
- [rfc/dtss-pystat.md](rfc/dtss-pystat.md) — Slot 等级与 PyStat Pass

---

## 2.0 之后、2.5 之前（可选中间里程碑）

| 主题 | 说明 |
|------|------|
| S2 boxed 完整堆模型 | GA+1，C backend 扩展 |
| Nuitka backend 正式接入 | `sikuwa build --backend nuitka` |
| 增量编译 / `.sikuwa` 缓存 | pir / pystat / build 分层缓存 |

以上不阻塞 2.5 LLVM IR 主线，可并行或裁剪。
