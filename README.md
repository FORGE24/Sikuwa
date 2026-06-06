# Sikuwa

**Sikuwa 2.0.0** — Python 构建与编译工具链  
**代号**：Sikuwa 2026/6/6 **Ver.A2**

---

## 状态

| 分支 | 说明 |
|------|------|
| **2.0 (Rust)** | 自研 PythonIR + PyStat + Native Codegen（进行中） |
| **1.x (Python)** | 根目录遗留代码，维护模式 |

当前里程碑：**Plan 1** — 工程底座 + PythonIR 骨架。

---

## 快速开始（2.0 Rust CLI）

### 环境

- Rust **1.75+**（见 `rust-toolchain.toml`）

### 构建

```bash
cargo build --release
```

### 命令

```bash
cargo run -- version
cargo run -- doctor
cargo run -- pir build tests/fixtures/add.py
cargo run -- pir text tests/fixtures/clamp.py
cargo run -- pir verify
cargo run -- validate -c sikuwa.a2.toml
```

---

## 架构概览

```text
.py → PythonIR (.pirb) → PyStat (.pstat) → Sikuwa-C → dll/so/exe
                              ↑
                         Nuitka Backend（可选，Plan 2+）
```

详见 [docs/rfc/](docs/rfc/)。

---

## 与 PyInstaller / Nuitka

| 工具 | 角色 |
|------|------|
| PyInstaller | 打包解释器 + 字节码 |
| Nuitka | Python 编译器（2.0 可选 Backend） |
| **Sikuwa** | 构建平台 + 自研 IR + 类型静态化 |

---

## 许可证

MIT — 见 [LICENSE](LICENSE)
