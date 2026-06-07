# x86_64 Assembly Hotpaths

Platform-specific hand-written assembly for Sikuwa runtime hot paths.

| Directory | Toolchain | ABI |
|-----------|-----------|-----|
| `linux/` | GAS (`.S`) | System V AMD64 |
| **`win-gnu/`** | **GAS (`.S`) — Windows 默认** | Windows x64 (MinGW gcc) |
| `win/` | MASM (`.asm`, `ml64`) | Windows x64（仅 `SKW_USE_MSVC=1`） |

## Windows：MinGW（推荐）

安装 [MSYS2](https://www.msys2.org/) 后：

```bash
pacman -S mingw-w64-ucrt-x86_64-gcc
```

将 `C:\msys64\ucrt64\bin`（或 `mingw64\bin`）加入 PATH，或设置：

```powershell
$env:CC = "C:\msys64\ucrt64\bin\gcc.exe"
```

验证：

```powershell
cargo run -- doctor
powershell -File scripts/asm-smoke.ps1
```

`sikuwa link shared` 在 Windows 上**默认**使用 MinGW gcc + `win-gnu/*.S`，不再依赖 ml64。

## Linux

```bash
bash scripts/asm-smoke.sh
```

## Symbols

| Symbol | Purpose |
|--------|---------|
| `skw_hash64` | Fast 64-bit hash (cache keys; not blake3) |
| `skw_i64_add_checked` | Integer add with overflow → `SKW_ERR_RANGE` |
| `skw_tagged_as_i64` | Extract `SKW_TAG_INT` from `skw_tagged_t` |

C fallbacks and `*_c` reference implementations live in `c/src/hotpath/dispatch.c`.
When asm objects are linked, compile with `-DSKW_HOTPATH_ASM`.

## MSVC 可选路径

```bat
set SKW_USE_MSVC=1
set CC=cl
ml64 /c /Fo hash.obj asm\x86_64\win\hash.asm
...
```

仅在显式设置 `SKW_USE_MSVC=1` 时使用 `win/*.asm` + ml64。
