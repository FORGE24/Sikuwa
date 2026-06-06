# Plan 5 — Python shim / @c_extern / struct codegen / manifest imports

| 状态 | Ver.A2 Plan 5 ✅ |
|------|------------------|

## 1. `@c_extern` / `@c_include`

Python 注释指令（PIR 层解析）：

```python
# skw @c_include string.h
# skw @c_extern libc strlen int64 s
# skw @c_extern libc.strlen(s) -> int64

def byte_len(s):
    return strlen(s)
```

- 模块字段：`Module.externs`、`Module.c_includes`
- Opcode：`CallExtern`
- Codegen：直接 C 调用 `strlen(...)`

## 2. 跨模块 manifest imports

```python
from add import add

def twice(a, b):
    return add(a, b)
```

- `Module.imports`：`ModuleImport { module, symbol, local }`
- Call → `OpOperand::Symbol("add.add")`
- `.skw.json` → `imports[]`：`{ module, symbol, c_symbol, kind }`

## 3. 闭包 / 类 struct codegen

生成 typedef（`.h`）：

```c
typedef struct skw_plan3_Point { int64_t x; int64_t y; } skw_plan3_Point_t;
typedef struct skw_plan3_make_adder_env { int64_t n; } skw_plan3_make_adder_env_t;
```

默认开启；`codegen c --no-structs` 可跳过。

## 4. Python embed shim

```bash
cargo run -- codegen c tests/fixtures/add.py --out-dir out/ --python-shim
# → out/add_pywrap.c  (PyInit_add / PyMethodDef)
```

- 头文件：`c/include/sikuwa/py_shim.h`
- 运行时：`c/src/py_shim/box.c`（`skw_py_unbox_i64` / `skw_py_box_i64`）
- 配置：`[sikuwa.ffi] python_shim = false`

## 用法

```bash
cargo run -- codegen c tests/fixtures/plan5_extern.py -o .sikuwa/build/extern
cargo run -- codegen c tests/fixtures/plan5_caller.py -o .sikuwa/build/caller
cargo run -- codegen c tests/fixtures/plan3.py --python-shim -o out/
```

## Plan 6 预留

- 闭包 struct 与 `MakeClosure` C 初始化连线
- `@c_extern` 参数类型 `str` → `const char*`
- 多模块 `skw link` + manifest 解析链接
- 完整 CPython 嵌入主程序模板

## 参考

- [rfc/native-c-ffi.md](rfc/native-c-ffi.md)
- [PLAN4.md](PLAN4.md)
