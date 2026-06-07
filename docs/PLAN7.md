# Plan 7 — Pass1 事实注入 + ABI 守卫

**依赖**：Plan 6（HPGI / 黄金管线）

## 阶段一：Pass1 — `# skw @type` ✅

在 PIR lower 阶段解析类型指令，写入 `Module.type_hints`，PyStat 分析时 seed HPGI。

### 语法

```python
# 完整签名（推荐）
# skw @type add int int -> int
def add(a, b):
    return a + b

# 紧邻 def 前的参数/返回注解
# skw @type a int
# skw @type b int
# skw @type -> int
def add(a, b):
    ...

# 下一函数返回类型
# skw @type int
def f():
    return 1
```

支持类型关键字：`int`/`int64`、`float`/`float64`、`bool`、`str`、`none`、`dyn`。

**优先级**：`# skw @type` > PEP 484 内联注解 > 同目录 `{stem}.pyi`。

### 诊断

- **SKW-T001** — 注解与推断冲突（见 `sikuwa-pystat` `check_return_hint`）

## 阶段二：PEP 484 + `.pyi` ✅

- `crates/sikuwa-pir/src/lower/pep484.rs` — 从 AST 采集 `def f(x: int) -> int`
- 自动加载 `{stem}.pyi`（若存在）
- `type_evidence.rs` 合并三层证据

```bash
cargo test -p sikuwa-pir pep484
cargo test -p sikuwa-pystat pyi_stub
```

## 阶段三：`@c_extern` 参数类型 ✅

```python
# skw @c_extern libc strlen int64 s:str
# skw @c_extern libc.strlen(s: str) -> int64
# skw @c_extern libc memcpy int64 dst:int64 src:int64 n:size_t
```

未标注参数：默认 `int64`；常见名 `s`/`text`/`msg` 推断为 `str`。

## 阶段四：SKW-T003 ABI 守卫 ✅

`codegen c` 在写入 `{stem}.skw.json` 前，与已有 manifest 比对：

- 相同 `source_hash` → 跳过
- 相同 `c_symbol` 的 export：`slot`、返回类型、参数个数/类型变化 → **SKW-T003**

```bash
cargo run -- codegen c tests/fixtures/plan7_types.py --out-dir out/ --allow-abi-break
```

## 阶段五：`pystat verify` ✅

独立验证类型诊断 + ABI（不生成 C）：

```bash
cargo run -- pystat verify tests/fixtures/plan7_types.py
cargo run -- pystat verify tests/fixtures/plan7_types.py --manifest out/plan7_types.skw.json
cargo run -- pystat verify tests/fixtures/plan7_types.py --allow-abi-break
cargo run -- pystat verify tests/fixtures/plan7_types.py --allow-type-warnings
```

## 待办

- [x] manifest `abi_breaking: true` 字段（RFC native-c-ffi）
- [x] `pystat verify --preset ci` + CI workflow

## manifest `abi_breaking`

当 `codegen c --allow-abi-break` 且相对已有 manifest 发生 ABI 变更时，受影响 export 写入：

```json
{
  "symbol": "add.add",
  "c_symbol": "skw_add_add",
  "abi_breaking": true
}
```

正常构建 omit 该字段（默认 `false`）。

## CI preset

Golden manifests：`tests/golden/manifests/{stem}.skw.json`  
用例列表：`tests/golden/manifests/preset.txt`

```bash
# 本地 / CI
bash scripts/pystat-verify-ci.sh
cargo run -- pystat verify --preset ci --all
cargo run -- pystat verify --preset ci tests/fixtures/add.py
```

`.github/workflows/ci-rust.yml` 在 smoke CLI 后运行 `pystat-verify-ci.sh`。

## 参考

- [rfc/dtss-pystat.md](rfc/dtss-pystat.md)
- [PLAN6.md](PLAN6.md)
