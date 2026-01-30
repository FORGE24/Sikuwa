# 🚀 Sikuwa - Python 项目编译打包工具

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen?style=flat-square)](https://github.com/FORGE24/Sikuwa/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Python 3.7+](https://img.shields.io/badge/python-3.7%2B-blue?style=flat-square)](https://www.python.org/downloads/)
[![Platform: Windows | Linux | macOS](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-brightgreen?style=flat-square)](https://github.com/FORGE24/Sikuwa)
[![Latest Release](https://img.shields.io/badge/release-v1.4.0-blue?style=flat-square)](https://github.com/FORGE24/Sikuwa/releases)

<div align="center">
  <p>
    <b>将 Python 代码编译为独立的可执行文件和动态链接库</b>
  </p>
  <p>
    <a href="#快速开始">快速开始</a> •
    <a href="#功能特性">功能特性</a> •
    <a href="#文档">文档</a> •
    <a href="#贡献">贡献</a> •
    <a href="#许可证">许可证</a>
  </p>
</div>

---

## 📋 目录

- [简介](#简介)
- [快速开始](#快速开始)
- [功能特性](#功能特性)
- [安装](#安装)
- [使用指南](#使用指南)
- [文档](#文档)
- [贡献](#贡献)
- [许可证](#许可证)

## 简介

**Sikuwa** 是一款强大的 Python 项目打包和编译工具，支持两种编译模式，专注于提供简单高效的跨平台编译解决方案。

### 💡 核心理念

通过配置化管理和自动化流程，将 Python 项目转换为独立可执行文件，支持 **Windows、Linux 和 macOS** 多平台。

### 🎯 编译模式

| 模式 | 工作流程 | 适用场景 | 输出格式 |
|------|--------|--------|--------|
| **Nuitka** | Python → 机器码 | 通用场景 | `.exe` / 二进制文件 |
| **Native** | Python → C/C++ → 机器码 | C/C++ 集成 | `.dll`/`.so`/`.dylib` + `.exe` |

## 快速开始

### 1️⃣ 安装

#### 方式一：预编译版本（推荐）
```bash
# 从 Releases 下载对应平台的预编译包
# 解压后添加到 PATH 环境变量
sikuwa --version
```

#### 方式二：源码安装
```bash
git clone https://github.com/FORGE24/Sikuwa.git
cd Sikuwa
pip install -e .
```

### 2️⃣ 初始化项目
```bash
# 创建默认配置文件 (sikuwa.toml)
sikuwa init

# 或创建自定义配置文件
sikuwa init -o my_config.toml
```

### 3️⃣ 构建项目
```bash
# 使用 Nuitka 构建（默认）
sikuwa build

# 使用 Native 模式构建
sikuwa build -m native

# 构建特定平台
sikuwa build -p windows

# 详细输出模式
sikuwa build -v
```

### ✅ 验证结果
```bash
# 检查输出目录
ls dist/

# 运行生成的可执行文件
./dist/项目名-<平台>/项目名.exe  # Windows
./dist/项目名-<平台>/项目名      # Linux/macOS
```

## 功能特性

### ⚡ 核心优势

- ✨ **双重编译模式** - Nuitka 和原生 C/C++ 编译可选
- 🔗 **通用链接库** - Native 模式生成标准 dll/so，兼容其他语言调用
- 🌍 **跨平台支持** - Windows、Linux、macOS 完全支持
- ⚙️ **灵活配置** - TOML 配置文件，轻松定制编译参数
- 🔍 **智能缓存 V1.2** - 编译即缓存，缓存命中 <1.5 秒
- ⚡ **减量编译** - 只编译变更代码，构建速度提升 10 倍+
- 📊 **详细日志** - 生成构建日志和清单，便于版本管理

### 📦 版本特性

#### v1.4.0 - 原生编译模式
- Python 源码 → C/C++ 源码 → 机器码
- 支持 GCC/Clang/MSVC 编译器
- 可选静态/动态链接 Python 库
- 完整的 Python 运行时嵌入支持

#### v1.3.0 - 智能缓存系统
- 基于 C++ 实现的高性能 LRU 缓存
- 预测缓存预热，后台异步编译
- 减量编译，构建速度提升 10 倍+
- 编译历史全记录，热点单元追踪

#### v1.2.0 - 基础功能
- 完整的项目初始化与配置管理
- 多平台编译支持（Windows/Linux/macOS）
- 环境检查与依赖验证
- 构建清单自动生成

## 安装

### 📥 预编译版（推荐）

1. **下载** - 从 [Releases](https://github.com/FORGE24/Sikuwa/releases) 获取对应平台的包
2. **解压** - 到本地目录（如 `C:\sikuwa` 或 `~/sikuwa`）
3. **配置 PATH**
   ```bash
   # Windows (PowerShell)
   $env:Path += ";C:\sikuwa"
   
   # Linux/macOS
   export PATH=$PATH:~/sikuwa
   ```
4. **验证**
   ```bash
   sikuwa --version
   ```

### 📚 源码版（开发者）

```bash
# 克隆仓库
git clone https://github.com/FORGE24/Sikuwa.git
cd Sikuwa

# 安装依赖
pip install -r requirements.txt

# 开发模式安装
pip install -e .

# 验证
python -m sikuwa --version
```

### 📋 系统要求

| 组件 | 版本 | 备注 |
|------|------|------|
| Python | 3.7+ | 必需 |
| **Windows** | - | MinGW-w64 8.1.0+ 或 MSVC 2019+ |
| **Linux** | - | GCC 7.3+ |
| **macOS** | - | Xcode Command Line Tools |

## 使用指南

### 🎯 预编译版

#### 初始化配置
```bash
# 创建默认配置
sikuwa init

# 自定义配置文件
sikuwa init -o my_config.toml --force
```

#### 构建项目
```bash
# Nuitka 模式（默认）
sikuwa build

# Native 模式（新增）
sikuwa build -m native

# 特定平台
sikuwa build -p windows
sikuwa build -p linux
sikuwa build -p macos

# 详细输出
sikuwa build -v

# 强制重新构建
sikuwa build --force
```

#### 其他命令
```bash
# 项目信息
sikuwa info

# 环境检查
sikuwa doctor

# 清理构建
sikuwa clean

# 帮助信息
sikuwa --help
sikuwa build --help
```

### 🐍 源码版（Python API）

#### 基础构建
```python
from sikuwa.config import ConfigManager
from sikuwa.builder import SikuwaBuilder

# 加载配置
config = ConfigManager.load_config("sikuwa.toml")

# 创建构建器
builder = SikuwaBuilder(config, verbose=True)

# 执行构建
builder.build()

# 特定平台构建
builder.build(platform="windows")
```

#### 自定义配置
```python
from sikuwa.config import BuildConfig, NuitkaOptions
from sikuwa.builder import SikuwaBuilder

nuitka_options = NuitkaOptions(
    standalone=True,
    onefile=True,
    enable_console=False,
    windows_icon="app_icon.ico"
)

config = BuildConfig(
    project_name="my_app",
    main_script="main.py",
    version="1.0.0",
    platforms=["windows", "linux"],
    nuitka_options=nuitka_options,
    resources=["data/*"]
)

builder = SikuwaBuilder(config)
builder.build(force=True)
```

#### 清理与管理
```python
from sikuwa.builder import SikuwaBuilder

builder = SikuwaBuilder(config)
builder.clean()              # 清理输出目录
builder._generate_manifest() # 生成清单文件
```

### 🔧 配置文件示例

#### Nuitka 模式
```toml
[sikuwa]
project_name = "my_project"
main_script = "main.py"
version = "1.0.0"
platforms = ["windows", "linux", "macos"]

[sikuwa.nuitka]
standalone = true
onefile = true
enable_console = true
follow_imports = true
```

#### Native 模式
```toml
[sikuwa]
project_name = "my_project"
main_script = "main.py"
version = "1.0.0"
compiler_mode = "native"

[sikuwa.native]
cc = "gcc"
cxx = "g++"
c_flags = ["-O2", "-fPIC"]
output_dll = true
output_exe = true
lto = true
strip = true
```

### 📂 输出目录结构

#### Nuitka 模式
```
dist/
├── my_project-windows/
│   ├── my_project.exe
│   ├── python*.dll
│   └── ...
├── my_project-linux/
│   ├── my_project
│   └── ...
└── my_project-macos/
    ├── my_project
    └── ...
```

#### Native 模式
```
dist/
├── native-windows/
│   ├── my_project.dll      # 动态链接库
│   ├── my_project.exe      # 可执行文件
│   └── my_project.lib      # 导入库
├── native-linux/
│   ├── libmy_project.so    # 共享库
│   └── my_project          # 可执行文件
└── native-macos/
    ├── libmy_project.dylib # 动态库
    └── my_project          # 可执行文件
```

## 编译指南

### 前置条件

- Python 3.7 或更高版本
- 系统编译器：
  - Windows：MinGW-w64 (8.1.0+) 或 MSVC (2019+)
  - Linux：GCC (7.3+)
  - macOS：Xcode Command Line Tools

### 依赖包安装
```bash
pip install nuitka click tomli tomli_w cython
```

### 编译步骤

1. **准备配置文件**
   ```bash
   sikuwa init
   # 编辑 sikuwa.toml
   ```

2. **检查环境**
   ```bash
   sikuwa doctor
   ```

3. **执行编译**
   ```bash
   # Nuitka 模式
   sikuwa build
   
   # Native 模式
   sikuwa build -m native
   ```

4. **查看输出**
   ```bash
   ls dist/
   ```

### 自举指南

自举是指使用 Sikuwa 编译自身源代码，生成独立可执行文件。

```bash
# 1. 获取源代码
git clone https://github.com/FORGE24/Sikuwa.git
cd Sikuwa

# 2. 安装依赖
pip install -r requirements.txt

# 3. 检查环境
python -m sikuwa doctor

# 4. 初始化配置
python -m sikuwa init

# 5. 执行自举编译
python -m sikuwa build -v

# 6. 验证结果
./dist/sikuwa-<platform>/sikuwa --version
```

## 文档

- 📖 [完整编译指南](docs/COMPILE_GUIDE.md)
- 🔧 [配置文件参考](docs/CONFIG_REFERENCE.md)
- 🚀 [自举指南](docs/BOOTSTRAP_GUIDE.md)
- 📚 [API 文档](docs/API.md)
- ❓ [常见问题](docs/FAQ.md)

## 贡献

欢迎贡献！请遵循以下步骤：

### 1. Fork 仓库
在 GitHub 上点击 Fork 按钮

### 2. 创建分支
```bash
git clone https://github.com/YOUR_USERNAME/Sikuwa.git
cd Sikuwa
git checkout -b feature/your-feature
```

### 3. 提交更改
```bash
git add .
git commit -m "feat: add your feature"
git push origin feature/your-feature
```

### 4. 创建 Pull Request
在 GitHub 上创建 PR，详细描述你的改进。

### 📋 贡献指南

- ✅ 遵循代码风格（使用 `black` 格式化）
- ✅ 添加单元测试
- ✅ 更新相关文档
- ✅ 在 PR 中清晰描述改动

## 常见问题

**Q: Sikuwa 和 PyInstaller 有什么区别？**

A: Sikuwa 支持两种编译模式（Nuitka 和 Native），提供更灵活的编译选项和更好的性能优化，特别是智能缓存和减量编译功能。

**Q: 编译后的文件大小会很大吗？**

A: Native 模式输出较小，Nuitka 模式可通过配置优化大小，通常为 10-100MB。

**Q: 支持哪些 Python 版本？**

A: 支持 Python 3.7+，推荐 Python 3.8 及以上。

## 许可证

本项目采用 **MIT 许可证** - 详见 [LICENSE](LICENSE) 文件

---

<div align="center">

### ⭐ 如果你觉得有帮助，请给个 Star！

[GitHub](https://github.com/FORGE24/Sikuwa) • [Issues](https://github.com/FORGE24/Sikuwa/issues) • [Discussions](https://github.com/FORGE24/Sikuwa/discussions)

Made with ❤️ by [FORGE24](https://github.com/FORGE24)

</div>
