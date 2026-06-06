# Plan 1 — 工程底座（Ver.A2）

**目标日期**：2026-03  
**GA 锚点**：2026-06-06

## 已完成（本 PR）

- [x] Cargo workspace（`sikuwa-core`, `sikuwa-config`, `sikuwa-pir`, `sikuwa-cli`）
- [x] PythonIR v1 骨架：`OpCode`, SSA `Module`, `.pirb` 编解码, `verify`
- [x] CLI：`version`, `doctor`, `pir verify|sample|dump`, `validate`
- [x] 配置 schema v2 子集（`sikuwa.a2.toml`）
- [x] RFC 文档三份（`docs/rfc/`）
- [x] Rust CI（`.github/workflows/ci-rust.yml`）

# Plan 1 — 工程底座（Ver.A2）

**目标日期**：2026-03  
**GA 锚点**：2026-06-06

## 已完成

- [x] Cargo workspace（`sikuwa-core`, `sikuwa-config`, `sikuwa-pir`, `sikuwa-cli`）
- [x] PythonIR v1 骨架：`OpCode`, SSA `Module`, `.pirb` 编解码, `verify`
- [x] CLI：`version`, `doctor`, `pir verify|sample|dump`, `validate`
- [x] 配置 schema v2 子集（`sikuwa.a2.toml`）
- [x] RFC 文档三份（`docs/rfc/`）
- [x] Rust CI（`.github/workflows/ci-rust.yml`）

## Plan 2（已完成）

见 [PLAN2.md](./PLAN2.md) — AST → PIR Lowering 原型。

## Plan 3 预览

- Sikuwa-C codegen
- 增量主路径
- 替换 1.x embed Native

## 本地开发

```bash
cargo test --all
cargo run -- pir verify
```

1.x Python 代码仍保留在仓库根目录，待 Plan 2 移入 `legacy/python/`。
