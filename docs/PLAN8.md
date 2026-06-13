# Plan 8 — Ver.A2 GA：PyStat 全 Pass + 闭包/Runtime 闭环

**目标版本**：`2.0.0`（GA）或先行 `2.0.0-beta.1`  
**前置**：Plan 1–7 ✅（PIR、HPGI 黄金管线、Pass1 事实、ABI 守卫）  
**GA 锚点（叙事）**：2026-06-06 Ver.A2 — 以 **可链接、可验证、可复现** 的端到端样例为验收

---

## 验收标准（GA Definition of Done）

| # | 标准 | 验证命令 |
|---|------|----------|
| G1 | PyStat Pass0–Pass5 全部落地，`[sikuwa.pystat]` 配置生效 | `cargo test -p sikuwa-pystat` + 配置 fixture |
| G2 | S0/S1/S3 在 codegen 中有对应 C 类型 emit | `clamp`→S1、`dyn` 路径→S3 harness |
| G3 | 闭包 `make_adder` 类 fixture **lower→pystat→codegen→link→C 调用** 闭环 | `scripts/closure-smoke.sh` |
| G4 | 跨模块 `from add import add` **link 成功** | `plan5_caller` + manifest imports |
| G5 | 诊断码 T001–T005 有测试；CI preset 覆盖 | `pystat verify --preset ci --all` |
| G6 | 一条命令构建 | `sikuwa build tests/fixtures/add.py -o dist/` |

---

## 现状 vs 缺口

```text
Pass0  PIR lower/verify              ✅
Pass1  @type / PEP484 / .pyi         ✅  (Plan 7)
Pass2  局部推断                      ✅  strict/min_slot + SKW-T002
Pass3  流敏感 narrow                 ✅  SKW-T004
Pass4  约束求解                        ✅  Call arity + SCC fixpoint
Pass5  降级决策                        ✅  materialize + config + SKW-T005
Codegen S0                           ✅  int/bool 静态函数
Codegen S1/S3                        ✅  skw_tagged_t / skw_value_t *
Closure runtime                      ✅  MakeClosure / LoadCell smoke
Multi-module link                    ✅  sikuwa build + extra_source_dirs
GA beta                              ✅  2.0.0-beta.1 — G1–G6 smoke
```

---

## 阶段一：Plan 8a — PyStat Pass2–Pass5（3–4 周）

### 8a.1 Pass2 — 局部推断（收口，~1 周）

已有：`SparseEnvironment`、`analyze_func`、跨过程 SCC。

| 任务 | 产出 |
|------|------|
| 显式 `Pass2` 模块边界 | `pystat/src/pass2.rs`，从 `analyze.rs` 抽离 seed+body infer |
| `mode = strict\|progressive\|compat` | strict：`SKW-T002` 无法 S0 则失败；progressive：降级 |
| `[sikuwa.pystat] min_slot` | 读 `sikuwa-config`，Pass5 前先做 floor 检查 |
| fixture | `tests/fixtures/pystat_strict.py`、`pystat_union.py` |

### 8a.2 Pass3 — 流敏感 narrow（~1 周）

| 任务 | 产出 |
|------|------|
| 分支边标注 | `CompareEq`/`CompareIs` + `Const` → narrow slot 类型 |
| `if isinstance(x, int)` 模式（可选） | 识别常见 guard，meet 分支出口 |
| `SKW-T004` | 动态 opcode 强制 dyn 时报告 |
| fixture | `tests/fixtures/narrow_if.py` — if 后 x 为 int |

### 8a.3 Pass4 — 约束求解（~1 周）

| 任务 | 产出 |
|------|------|
| 局部约束边 | `Call` 实参 → 形参 join；`BinOp` 传播 |
| 与 SCC 统一 | `apply_fixpoint` 输出 `FuncSummary` + 约束违反列表 |
| 单元测试 | 互递归 + 参数 contravariance 边界 case |

### 8a.4 Pass5 — 降级决策（~3–5 天）

| 任务 | 产出 |
|------|------|
| `materialize_slot` + `min_slot` | `tagged`→至少 S1；`static`→拒绝 S3 除非 allow_dyn_fallback |
| manifest slot 字段 | 与 `FuncStat` 一致；`pystat verify` 校验 |
| `SKW-T005` | profile（strict/progressive）与结果 slot 不一致 |

**里程碑命令**：

```bash
cargo test -p sikuwa-pystat
cargo run -- pystat report tests/fixtures/narrow_if.py --json
```

---

## 阶段二：Plan 8b — Slot 分层 Codegen（2–3 周）

### 8b.1 S1 Tagged emit

| 任务 | 产出 |
|------|------|
| `PhysicalType` + `SlotLevel` → C 签名 | 参数/返回/局部用 `skw_tagged_t` |
| 栈布局 | RFC：栈上 `skw_tagged_t`（不用 pointer tagging） |
| `TaggedLayout` → tag 常量 | `skw_tagged_from_i64` / 分支 unpack |
| fixture | `clamp.py` → S1 函数签名 + asm `skw_tagged_as_i64` 路径 |

### 8b.2 S3 Dyn emit

| 任务 | 产出 |
|------|------|
| `LoadAttr`/`Subscript`/`Call` 未知 callee | `skw_value_t` + runtime API |
| 扩展 `value.c` | `skw_value_from_i64` 等已有；补 `skw_invoke` 桩 |
| dyn 跳板 | 可选 `skw_<mod>_<fn>_dyn` 符号（RFC SKW-T003 并存策略） |

### 8b.3 `@c_extern` 物理类型

| 任务 | 产出 |
|------|------|
| `str` → `skw_str_t` 或 `const char*` | 与 manifest param type 一致 |
| `CallExtern` codegen | 按 `ExternDecl.param_types` emit |

**里程碑**：

```bash
cargo run -- codegen c tests/fixtures/clamp.py --out-dir out/ --opt
# 检查 .h 签名含 skw_tagged_t
cargo run -- link shared out/ -o dist/libclamp.so
```

---

## 阶段三：Plan 8c — 闭包 / 类 Runtime 闭环（2–3 周）

参照 [rfc/native-c-ffi.md §6](rfc/native-c-ffi.md)。

### 8c.1 闭包

| 任务 | 产出 |
|------|------|
| `MakeClosure` emit | env struct 初始化 + `fn` 指针 |
| nested `FuncDef` | static 内部函数 + `SKW_CALL` 约定 |
| `LoadCell`/`StoreCell` | env 字段读写 |
| fixture | `tests/fixtures/plan3.py` `make_adder` |

### 8c.2 类（最小 GA）

| 任务 | 产出 |
|------|------|
| `BuildClass` / 方法 | `Point.__init__` + 方法 S0 emit |
| `self` 参数 | struct 指针首参 |
| 暂不追求 | 继承、descriptor、元类 |

### 8c.3 端到端 smoke

```bash
# 新增
scripts/closure-smoke.sh   # plan3 闭包
scripts/class-smoke.sh     # Point 类
```

**验收**：C harness 调用 `skw_*_make_adder(n)(x)` 返回正确值。

---

## 阶段四：Plan 8d — 多模块 + 一体化 Build（1–2 周）

| 任务 | 产出 |
|------|------|
| `link` 读 manifest `imports` | `-l` / 符号前缀解析 `skw_add_add` |
| 拓扑排序编译顺序 | caller 链 add 静态库或同库多模块 |
| **`sikuwa build`** | `lower → golden pipeline → codegen → link` |
| 缓存 | `.sikuwa/pir`、`.sikuwa/pystat`（可选 GA 后） |
| crate | `sikuwa-engine` 或 `cli build` 子命令 |

```bash
sikuwa build tests/fixtures/plan5_caller.py -o dist/ --opt
```

---

## 阶段五：Plan 8e — GA 封板（1 周）

| 任务 | 产出 |
|------|------|
| 版本 | `2.0.0-beta.1` → soak → `2.0.0` |
| 文档 | README、PLAN1–8 状态、RFC dtss Pass 状态 ✅ |
| CI | closure/class smoke + 扩展 golden manifests |
| 发布清单 | CHANGELOG、doctor 检查项、MinGW/ Linux 矩阵 |

---

## 推荐日历（6 × 2 周，全职）

|  Sprint | 日期（示例） | 焦点 | 出口 |
|--------|--------------|------|------|
| S1 | W1–W2 | 8a.1 Pass2 + 8a.2 Pass3 起步 | strict/narrow 测试绿 |
| S2 | W3–W4 | 8a.3–8a.4 Pass4–5 | T002/T004/T005 + config |
| S3 | W5–W6 | 8b.1 S1 tagged emit | clamp S1 链接通过 |
| S4 | W7–W8 | 8b.2–8b.3 S3 + extern | dyn 桩 + strlen str |
| S5 | W9–W10 | 8c 闭包/类 smoke | plan3 harness 绿 |
| S6 | W11–W12 | 8d build + 8e GA | G1–G6 全绿，打 tag |

业余节奏：上述 **×2**（约 **12–14 周**）。

---

## 依赖关系

```mermaid
flowchart LR
  P7[Plan 7 Pass1/ABI] --> P8a[8a PyStat Pass2-5]
  P8a --> P8b[8b Slot Codegen]
  P8b --> P8c[8c Closure/Class]
  P8c --> P8d[8d Multi-module Build]
  P8d --> P8e[8e GA Release]
  P6[Plan 6 HPGI/O1-O2] --> P8a
  P6 --> P8b
```

**关键路径**：Pass5 降级决策 → S1 emit → 闭包 env 布局（依赖 PGTE slot）→ 多模块 link。

---

## 风险与裁剪（若 GA 前时间不足）

| 可延后到 GA+1 | 不可砍 |
|---------------|--------|
| S2 boxed 完整堆模型 | Pass2–5 最小闭环 |
| Nuitka backend | S0+S1+S3 三档 emit 各 1 fixture |
| 完整 CPython embed 主程序 | 闭包 make_adder smoke |
| Pass3 isinstance 全模式 | 多模块 link + `sikuwa build` |

---

## 参考

- [PLAN6.md](PLAN6.md) — HPGI / 黄金管线
- [PLAN7.md](PLAN7.md) — Pass1 / ABI 守卫
- [rfc/dtss-pystat.md](rfc/dtss-pystat.md) — Pass 定义
- [rfc/native-c-ffi.md](rfc/native-c-ffi.md) — S0–S3、闭包 §6
- [ROADMAP.md](ROADMAP.md) — **2.5（2027）** 含 Python → LLVM IR 后端
