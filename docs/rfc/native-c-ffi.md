# RFC: 原生 C FFI（Sikuwa-C ABI）

| 字段 | 值 |
|------|-----|
| 状态 | Accepted (Plan 4) |
| 依赖 | PythonIR v1.2、PyStat/ITR、`sikuwa-codegen-c` |
| ABI 版本 | `SKW_ABI_1` |

## 摘要

Sikuwa 2.0 在 **PyStat slot 等级**（S0–S3）之上定义一层 **稳定、可链接、可动态加载** 的 C ABI。编译产物不仅是 `.c/.h` 源文件，还包括 **模块描述符、导出表、类型布局 manifest**，供：

- 外部 C/C++/Rust 直接链接或 `dlopen`
- 嵌入 Python 时的 shim（`sikuwa-py-bridge`，Plan 5+）
- 跨模块静态链接（PGTE 跨文件类型图）

设计原则：**S0 零开销、S3 统一入口、ABI 少而稳、实现可演进**。

---

## 1. 分层结构

```text
┌─────────────────────────────────────────────────────────┐
│  宿主 / 第三方 C/C++/Rust                                │
└───────────────────────────┬─────────────────────────────┘
                            │ skw_* 符号 / skw_module_t
┌───────────────────────────▼─────────────────────────────┐
│  Layer A — ABI 契约          c/include/sikuwa/abi.h      │
│  版本宏、导出属性、调用约定、错误码                        │
├─────────────────────────────────────────────────────────┤
│  Layer B — 运行时值模型      c/include/sikuwa/runtime.h  │
│  sikuwa_value_t / tagged / str slice / 异常边界           │
├─────────────────────────────────────────────────────────┤
│  Layer C — 模块接口（生成）   out/<mod>/<mod>.h + .c     │
│  S0 直调函数、类 struct、闭包 env、dyn 跳板               │
├─────────────────────────────────────────────────────────┤
│  Layer D — 模块元数据（生成） out/<mod>/<mod>.skw.json   │
│  符号表、slot 等级、ITR 布局、依赖边（PGTE 导出）        │
└─────────────────────────────────────────────────────────┘
```

| 层 | 稳定性 | 维护方 |
|----|--------|--------|
| A | 长期冻结（仅增不删） | Sikuwa 仓库 `c/include/sikuwa/` |
| B | 大版本可扩展字段 | 同上 |
| C/D | 每次编译生成 | `sikuwa-codegen-c` |

---

## 2. ABI 头文件（Layer A）

### 2.1 版本与平台

```c
/* c/include/sikuwa/abi.h */
#define SKW_ABI_VERSION_MAJOR 1
#define SKW_ABI_VERSION_MINOR 0
#define SKW_ABI_STRING "1.0"

#if defined(_WIN32)
  #define SKW_EXPORT __declspec(dllexport)
  #define SKW_IMPORT __declspec(dllimport)
  #define SKW_CALL   __cdecl
#else
  #define SKW_EXPORT __attribute__((visibility("default")))
  #define SKW_IMPORT
  #define SKW_CALL
#endif

#ifdef SKW_BUILDING_MODULE
  #define SKW_API SKW_EXPORT
#else
  #define SKW_API SKW_IMPORT
#endif
```

### 2.2 错误与状态（跨边界不用 C++ 异常）

```c
typedef enum skw_status {
    SKW_OK = 0,
    SKW_ERR_TYPE = 1,      /* 动态类型不匹配 */
    SKW_ERR_RANGE = 2,
    SKW_ERR_OOM = 3,
    SKW_ERR_UNREACHABLE = 4,
    SKW_ERR_PYTHON = 5,    /* Py 桥接失败 */
} skw_status_t;

typedef struct skw_result_i64 {
    skw_status_t status;
    int64_t value;
} skw_result_i64_t;
```

S0 热路径函数可返回裸 `int64_t`；带 `skw_result_*` 的变体用于 strict / 可诊断构建。

---

## 3. 值模型与 Slot 映射（Layer B）

PyStat `PhysicalType` + `SlotLevel` 映射到 C 类型：

| Slot | PyStat | C 传参类型 | FFI 说明 |
|------|--------|-----------|----------|
| S0 | `Int64` / `Bool` (ITR) | `int64_t` | 同槽 ITR，bool 用 0/1 |
| S0 | `Float64` | `double` | |
| S0 | `Str` | `skw_str_t` | 非 NUL 结尾 slice |
| S1 | tagged | `skw_tagged_t` | `{ uint8_t tag; union { ... } u }` |
| S2 | boxed | `skw_box_t*` | 堆对象头 + 类型 id |
| S3 | dyn | `skw_value_t` | 不透明句柄，`void*`  typedef（当前 codegen 占位） |

### 3.1 字符串（避免 Python `str` 与 C `char*` 混淆）

```c
typedef struct skw_str {
    const char *data;
    size_t len;
} skw_str_t;

/* 所有权：SKW_STR_BORROW | SKW_STR_OWNED，由 manifest 标注 */
```

### 3.2 Tagged（S1，为 ITR 回退预留）

```c
typedef enum skw_tag {
    SKW_TAG_NONE = 0,
    SKW_TAG_BOOL = 1,
    SKW_TAG_INT  = 2,
    SKW_TAG_FLOAT = 3,
    SKW_TAG_STR  = 4,
    SKW_TAG_OBJECT = 5,
} skw_tag_t;

typedef struct skw_tagged {
    skw_tag_t tag;
    union {
        int64_t i;
        double f;
        skw_str_t s;
        skw_value_t obj;
    } as;
} skw_tagged_t;
```

### 3.3 动态值（S3）

```c
typedef struct skw_value skw_value_t;  /* 不透明 */

SKW_API skw_value_t skw_value_from_i64(int64_t v);
SKW_API int64_t     skw_value_to_i64(skw_value_t v, skw_status_t *st);
SKW_API void        skw_value_retain(skw_value_t v);
SKW_API void        skw_value_release(skw_value_t v);
```

Plan 4 实现可先用 **intrusive refcount**；Plan 5+ 与 Python 对象桥接时对齐 `PyObject*`.

---

## 4. 符号命名与导出

### 4.1 命名规则

PIR `SymbolRef` → C 符号：

| PIR | C 符号 |
|-----|--------|
| `add.add` | `skw_add_add` |
| `plan3.Point.__init__` | `skw_plan3_Point___init__` |
| `plan3.make_adder.add` (nested) | `skw_plan3_make_adder_add` |

规则：`skw_` + 将 `.` 替换为 `_`，保留 Python 双下划线。

### 4.2 导出方式

**静态链接 / 单模块：**

```c
/* generated add.h */
#include <stdint.h>
#include "sikuwa/abi.h"

SKW_API int64_t skw_add_add(int64_t a, int64_t b);
```

**动态库：**

```c
/* generated add_module.c */
#include "sikuwa/module.h"

static skw_fn_entry_t skw_add_fns[] = {
    { "add.add", SKW_SLOT_S0, (void *)skw_add_add },
};

SKW_API const skw_module_t skw_module_add = {
    .abi_major = SKW_ABI_VERSION_MAJOR,
    .abi_minor = SKW_ABI_VERSION_MINOR,
    .name = "add",
    .source_hash = { /* blake3 32B */ },
    .fn_count = 1,
    .fns = skw_add_fns,
};
```

Windows 额外生成 `.def`：

```def
EXPORTS
    skw_add_add
    skw_module_add
```

---

## 5. 调用约定矩阵

| 场景 | 入口形态 | 示例 |
|------|----------|------|
| S0 静态函数 | 直接 C 函数 | `int64_t skw_add_add(int64_t, int64_t)` |
| S0 方法 | `self` 为首参 struct 指针 | `int64_t skw_plan3_Point_sum(skw_plan3_Point_t *self)` |
| S1 混合 | tagged 参数 | `skw_tagged_t skw_clamp(...)` |
| S3 dyn | 通用派发 | `skw_value_t skw_invoke(skw_sym_id, skw_value_t *argv, int argc)` |
| 闭包 | env struct + 函数指针 | 见 §6 |

所有 `SKW_API` 函数使用 **`SKW_CALL`（Windows `__cdecl`）**，与 Python C-API 默认一致，便于 shim。

---

## 6. 闭包、类、容器

### 6.1 闭包（对应 PIR `MakeClosure` / `LoadCell`）

```c
typedef struct skw_plan3_make_adder_env {
    int64_t n;   /* cell: PGTE 确定 ITR → int64_t */
} skw_plan3_make_adder_env_t;

typedef int64_t (SKW_CALL *skw_plan3_add_fn)(
    skw_plan3_make_adder_env_t *env,
    int64_t x);

typedef struct skw_plan3_make_adder_closure {
    skw_plan3_make_adder_env_t env;
    skw_plan3_add_fn fn;
} skw_plan3_make_adder_closure_t;

SKW_API skw_plan3_make_adder_closure_t skw_plan3_make_adder(int64_t n);
```

Codegen 从 `FuncDef.cellvars` + PyStat slot 布局生成 **env struct**；`fn` 指向同编译单元内的 static 函数。

### 6.2 类（对应 `ClassDef` / `BuildClass`）

```c
typedef struct skw_plan3_Point {
    int64_t x;
    int64_t y;
} skw_plan3_Point_t;

SKW_API void skw_plan3_Point___init__(skw_plan3_Point_t *self, int64_t x, int64_t y);
```

实例：**C struct 值语义**（S0）或 **堆分配 + 指针**（S2/S3），由 PyStat 对 `self.x` 等属性的 slot 决定。

### 6.3 下标 / 属性（dyn 回退）

S0 无法证明时降级：

```c
SKW_API skw_value_t skw_subscript_load(skw_value_t obj, skw_value_t key);
SKW_API skw_status_t skw_subscript_store(skw_value_t obj, skw_value_t key, skw_value_t val);
```

---

## 7. 模块描述符与 PGTE 互操作

`.skw.json` manifest（与 `.pstat` 对齐，供链接器 / Rust 主控读取）：

```json
{
  "abi": "1.0",
  "module": "add",
  "source_hash": "...",
  "exports": [
    {
      "symbol": "add.add",
      "c_symbol": "skw_add_add",
      "slot": "S0",
      "signature": {
        "params": [{"name":"a","type":"int64"},{"name":"b","type":"int64"}],
        "return": "int64"
      }
    }
  ],
  "imports": [],
  "itr_slots": [
    {"logical": "x", "physical": "int64", "strategy": "itr"}
  ]
}
```

**跨模块链接：**

- 链接期：manifest `imports` → `-l` / 符号解析
- PGTE 变更触发 `SKW-T003`：slot 从 S0→S3 时 **c_symbol 保留**，新增 `skw_add_add_dyn` 或在 manifest 标记 `abi_breaking: true`

---

## 8. 外部 C → Sikuwa / Python 双向 FFI

### 8.1 导出（Sikuwa → C）— 主路径

已实现雏形：`sikuwa codegen c` → `.h/.c`。Plan 4 扩展：

- 生成 `skw_module_t` + manifest
- `c/include/sikuwa/` 安装到 sysroot

### 8.2 导入（C → Sikuwa）— `@c_extern`（Plan 5）

PIR 扩展：

```text
@c_extern("libc", "strlen")
declare strlen(s: str) -> int64
```

Lowering → `LoadExtern` opcode → codegen `#include` + 直接调用。

### 8.3 Python 嵌入 shim（Plan 5）

```c
/* sikuwa/py_shim.h */
SKW_API PyObject *skw_py_call(const char *module_func, PyObject *args);
SKW_API int64_t   skw_unbox_i64(PyObject *o, skw_status_t *st);
```

嵌入模式配置（`sikuwa.a2.toml`）：

```toml
[sikuwa.ffi]
abi = "1.0"
export_dll = true
export_module_desc = true
python_shim = false   # true 时额外生成 _pywrap.c
visibility = "hidden" # 非导出符号 hidden
```

---

## 9. 内存、线程、异常边界

| 规则 | 说明 |
|------|------|
| 异常 | C ABI **不抛异常**；C++ 宿主需 `noexcept` 包装 |
| 字符串返回 | S0 `skw_str_t` 默认 **borrow**；生命周期 ≤ 调用栈 |
| 堆对象 | S2/S3 成对 `retain/release` |
| 线程 | Plan 4 运行时 **无 GIL**；Python shim 层持有 GIL |
| GC | 纯 S0 模块 **无 GC**；含 dyn 的模块链接 `libsikuwa_rt` |

---

## 10. 目录与构建集成

```text
c/
  include/sikuwa/
    abi.h
    runtime.h
    module.h
  src/
    runtime/
      value.c          # S3 dyn（Plan 4）
      tagged.c         # S1（Plan 4）
  sikuwa_cache/        # 已有规划
  sikuwa_rt/           # 静态运行时 lib

.sikuwa/build/
  add/
    add.h
    add.c
    add.skw.json
    add.def            # Windows
  libadd.so / add.dll
```

**Cargo / CLI（Plan 4）：**

```bash
sikuwa codegen c add.py --out-dir .sikuwa/build/add --ffi module,manifest
sikuwa link .sikuwa/build/add -o dist/libadd.so
```

---

## 11. 与现有 codegen 的差距（实施清单）

| 项 | 现状 | Plan 4 |
|----|------|--------|
| 符号前缀 | `add_add` | 统一 `skw_add_add` + `SKW_API` |
| 公共头 | 模块私有 `.h` | 依赖 `sikuwa/abi.h` |
| `sikuwa_value_t` | `void*` typedef | 不透明 struct + runtime |
| 多基本块 | 未支持 | CFG → C `goto`/结构化 |
| 闭包 / 类 | 仅 PIR | env struct + 方法 |
| manifest | 无 | `.skw.json` |
| dyn 跳板 | 无 | `skw_invoke` |

---

## 12. 验证策略

1. **ABI 兼容性测试**：`tests/ffi/abi_version.c` 编译期断言宏
2. **Round-trip**：C 宿主调用 `skw_add_add` ↔ Python ctypes 同签名
3. **Manifest 锁**：`.pstat` 与 `.skw.json` hash 一致
4. **跨平台**：CI 构建 `win64` / `linux-gnu` 各跑一次 `sikuwa link`

---

## 参考

- [a2-architecture.md](./a2-architecture.md)
- [dtss-pystat.md](./dtss-pystat.md)
- [../PLAN3.md](../PLAN3.md)
- Python C-API 调用约定：`__cdecl` / `PyObject*`
