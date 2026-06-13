# Sikuwa

**Sikuwa 2.0.0-beta.1** — Python 构建与编译工具链  
**代号**：Sikuwa 2026/6/6 **Ver.A2**

---

## 状态

| 分支 | 说明 |
|------|------|
| **2.0 (Rust)** | PythonIR + PyStat Pass0–5 + S0/S1/S3 Native Codegen + 闭包/多模块 build |
| **1.x (Python)** | 根目录遗留代码，维护模式 |

当前里程碑：**Plan 8 beta** — GA Definition of Done（G1–G6）已打通； soak 后打 `2.0.0` tag。

**后续版本**：[docs/ROADMAP.md](docs/ROADMAP.md) — **2.5（2027 / Ver.B1）** 目标含 **Python → LLVM IR** 后端（`sikuwa-codegen-llvm`）。

---

## 快速开始（2.0 Rust CLI）

### 环境

- Rust **1.75+**（见 `rust-toolchain.toml`）
- C 编译器（Linux/macOS: `gcc`；Windows: MinGW `gcc`）

### 构建

```bash
cargo build --release
```

### 命令

```bash
cargo run -- version
cargo run -- doctor
cargo run -- pir build tests/fixtures/add.py
cargo run -- pystat report tests/fixtures/narrow_if.py --json
cargo run -- pystat verify --preset ci --all
cargo run -- codegen c tests/fixtures/add.py --out-dir out/ --opt
cargo run -- build tests/fixtures/plan5_caller.py -o dist/ --opt
cargo run -- link shared out/ -o dist/libadd.so
cargo run -- validate -c sikuwa.toml
```

### Smoke 测试

```bash
bash scripts/ffi-smoke.sh          # add.py 端到端
bash scripts/closure-smoke.sh      # plan3 闭包
bash scripts/multimodule-smoke.sh  # plan5_caller 跨模块
```

---

## 架构概览

```text
.py → PythonIR (.pirb) → PyStat (.pstat) → Sikuwa-C → dll/so/exe
                              ↑
                         Nuitka Backend（可选，Plan 2+）
```

详见 [docs/rfc/](docs/rfc/) 与 [docs/PLAN8.md](docs/PLAN8.md)。

---

## 与 PyInstaller / Nuitka

| 工具 | 角色 |
|------|------|
| PyInstaller | 打包解释器 + 字节码 |
| Nuitka | Python 编译器（2.0 可选 Backend） |
| **Sikuwa** | 构建平台 + 自研 IR + 类型静态化 |

---

## 许可证

GPL v3 — 见 [LICENSE](LICENSE)（[中文参考译文](LICENSE.CHINESE)）
