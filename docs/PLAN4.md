# Plan 4 — 原生 C FFI（完成）

| 状态 | Ver.A2 Plan 4 ✅ |
|------|------------------|

## 交付清单

### Layer A/B — C ABI + Runtime

```text
c/include/sikuwa/
  abi.h       SKW_ABI_1、SKW_API、skw_status_t
  runtime.h   skw_value_t、skw_tagged_t、S3 API 声明
  module.h    skw_module_t 导出表

c/src/runtime/
  value.c     skw_value_from_i64 / to_i64 / release（Plan 4 最小 S3）
```

### Codegen（`sikuwa-codegen-c`）

- 符号：`skw_<module>_<func>`
- 头文件含 `sikuwa/abi.h`，`.c` 含 `SKW_BUILDING_MODULE`
- **`*.skw.json`** manifest（exports / ITR / source_hash）
- **`skw_module_t`** 模块描述符 + Windows `.def`
- **CFG codegen**：`if`/`while` → `goto skw_bb_*`（`clamp.py`）

### Link（`sikuwa-link` + CLI）

```bash
sikuwa codegen c tests/fixtures/add.py --out-dir .sikuwa/build/add
sikuwa link shared .sikuwa/build/add -o dist/libadd.so
# --no-runtime  跳过 value.c
# --cc gcc
```

- 自动查找 `c/include`，默认链入 `c/src/runtime/*.c`

### 验证

```text
tests/ffi/
  abi_version.c    ABI 版本编译期断言
  harness.c        调用 skw_add_add
  runtime_test.c   S3 value 往返

scripts/ffi-smoke.sh   CI（ubuntu）端到端
```

### 配置（schema v2）

```toml
[sikuwa.ffi]
abi = "1.0"
export_dll = true
export_module_desc = true
link_runtime = true
visibility = "hidden"
```

## 端到端管线

```text
.py → PIR → PyStat → codegen c → .h/.c/.skw.json
                              → link shared → lib*.so|.dll
                              → 外部 C / ctypes 调用 skw_* 
```

## Plan 5 预留

- Python embed shim（`sikuwa-py-bridge`）
- `@c_extern` 导入
- 闭包 env / 类 struct codegen
- 跨模块 PGTE manifest `imports`

## 参考

- [rfc/native-c-ffi.md](rfc/native-c-ffi.md)
- [PLAN3.md](PLAN3.md)
