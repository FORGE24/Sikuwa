# Sikuwa

<p align="center">
  <strong>基于 Nuitka 的跨平台 Python 项目打包工具</strong>
</p>

<p align="center">
  <a href="https://github.com/FORGE24/Sikuwa/releases"><img src="https://img.shields.io/github/v/release/FORGE24/Sikuwa?style=flat-square&logo=github" alt="GitHub Release"></a>
  <a href="https://github.com/FORGE24/Sikuwa/blob/main/LICENSE"><img src="https://img.shields.io/github/license/FORGE24/Sikuwa?style=flat-square" alt="License"></a>
  <a href="https://github.com/FORGE24/Sikuwa/stargazers"><img src="https://img.shields.io/github/stars/FORGE24/Sikuwa?style=flat-square&logo=github" alt="GitHub Stars"></a>
  <a href="https://github.com/FORGE24/Sikuwa/issues"><img src="https://img.shields.io/github/issues/FORGE24/Sikuwa?style=flat-square" alt="Issues"></a>
  <img src="https://img.shields.io/badge/python-3.7%2B-blue?style=flat-square&logo=python" alt="Python Version">
</p>

<p align="center">
  <a href="#features">功能特性</a> |
  <a href="#installation">安装</a> |
  <a href="#quick-start">快速开始</a> |
  <a href="#documentation">文档</a> |
  <a href="#contributing">贡献指南</a>
</p>

---

## 概述

Sikuwa 是一款专业的 Python 项目打包工具，基于 Nuitka 编译器构建，支持将 Python 项目编译为独立的可执行文件。提供两种编译模式，满足不同场景的需求。

### 核心特性

| 特性 | 描述 |
|------|------|
| 双模式编译 | 支持 Nuitka 模式和 Native 原生编译模式 |
| 跨平台支持 | Windows、Linux、macOS 全平台构建 |
| 增量编译 | 智能检测变更，仅编译修改的部分 |
| 配置驱动 | 基于 TOML 的声明式配置 |
| 国际化 | 内置多语言支持 (i18n) |
| 高性能缓存 | C++ 实现的智能缓存系统 |

---

## Features

### 编译模式

#### Nuitka 模式 (默认)

使用 Nuitka 编译器将 Python 代码编译为优化的机器码。

- 完整的 Python 兼容性
- 自动依赖分析与打包
- 支持 Standalone 和 OneFile 模式
- 内置插件系统

#### Native 模式

将 Python 代码转换为 C/C++ 源码，通过 GCC/G++ 编译为原生二进制文件。

- 生成通用动态链接库 (.dll/.so)
- 不依赖 Python 专用格式 (.pyd)
- 支持静态链接
- 可保留生成的 C/C++ 源码用于审计

### 增量编译系统

基于依赖图的智能增量编译，实现"指哪编哪"的精确编译策略。

```
源码改变 -> 依赖分析 -> 影响范围计算 -> 最小化重编译
```

- 函数级粒度的变更检测
- 依赖关系追踪
- 编译缓存持久化
- 并行编译支持

---

## Installation

### 系统要求

| 组件 | 版本要求 |
|------|----------|
| Python | >= 3.7 |
| Nuitka | >= 2.0 (自动安装) |
| GCC/G++ | >= 8.0 (Native 模式) |
| CMake | >= 3.16 (可选，用于 C++ 扩展) |

### 通过 pip 安装

```bash
pip install sikuwa
```

### 从源码安装

```bash
git clone https://github.com/FORGE24/Sikuwa.git
cd Sikuwa
pip install -e .
```

### 验证安装

```bash
sikuwa --version
sikuwa doctor
```

---

## Quick Start

### 1. 初始化项目

```bash
sikuwa init
```

该命令将在当前目录创建 `sikuwa.toml` 配置文件。

### 2. 配置项目

编辑 `sikuwa.toml`：

```toml
[sikuwa]
project_name = "myproject"
version = "1.0.0"
main_script = "src/main.py"
src_dir = "src"
output_dir = "dist"
platforms = ["windows", "linux"]

[sikuwa.nuitka]
standalone = true
onefile = false
include_packages = [
    "requests",
    "numpy",
]
```

### 3. 构建项目

```bash
# 使用 Nuitka 模式构建
sikuwa build

# 使用 Native 模式构建
sikuwa build -m native

# 构建指定平台
sikuwa build -p windows

# 详细输出模式
sikuwa build -v
```

### 4. 清理构建文件

```bash
sikuwa clean
```

---

## 命令行参考

### 命令列表

| 命令 | 描述 |
|------|------|
| `build` | 构建项目 |
| `clean` | 清理构建文件 |
| `init` | 初始化项目配置 |
| `info` | 显示项目信息 |
| `validate` | 验证配置文件 |
| `doctor` | 检查构建环境 |
| `version` | 显示版本信息 |

### build 命令选项

| 选项 | 简写 | 描述 |
|------|------|------|
| `--config` | `-c` | 指定配置文件路径 |
| `--platform` | `-p` | 目标平台 (windows/linux/macos) |
| `--mode` | `-m` | 编译模式 (nuitka/native) |
| `--verbose` | `-v` | 详细输出模式 |
| `--force` | `-f` | 强制重新构建 |
| `--keep-c-source` | - | 保留生成的 C/C++ 源码 |

---

## 配置文件详解

### 基础配置

```toml
[sikuwa]
# 项目基本信息
project_name = "myproject"      # 项目名称
version = "1.0.0"               # 版本号
description = "项目描述"         # 可选
author = "作者名"               # 可选

# 构建路径配置
main_script = "main.py"         # 入口脚本
src_dir = "."                   # 源码目录
output_dir = "dist"             # 输出目录
build_dir = "build"             # 构建临时目录

# 目标平台
platforms = ["windows", "linux", "macos"]
```

### Nuitka 选项

```toml
[sikuwa.nuitka]
# 基础选项
standalone = true               # 独立模式
onefile = false                 # 单文件模式
follow_imports = true           # 跟踪导入
show_progress = true            # 显示进度
enable_console = true           # 启用控制台

# 优化选项
optimize = true                 # 启用优化
lto = false                     # 链接时优化

# Windows 特定选项
windows_icon = "icon.ico"
windows_company_name = "Company"
windows_product_name = "Product"

# macOS 特定选项
macos_app_bundle = false
macos_icon = "icon.icns"

# 包含/排除
include_packages = ["package1", "package2"]
include_modules = ["module1"]
nofollow_imports = ["test_*"]

# 插件
enable_plugins = ["pyside6", "numpy"]

# 数据文件
include_data_dirs = [
    { src = "assets", dest = "assets" }
]

# 额外参数
extra_args = [
    "--assume-yes-for-downloads"
]
```

### Native 编译选项

```toml
[sikuwa.native]
# 编译器选择
cc = "gcc"                      # C 编译器
cxx = "g++"                     # C++ 编译器

# 编译标志
c_flags = ["-O2", "-fPIC"]
cxx_flags = ["-O2", "-fPIC", "-std=c++17"]
link_flags = []

# 输出选项
output_dll = true               # 生成动态库
output_exe = true               # 生成可执行文件
output_static = false           # 生成静态库

# Python 嵌入
embed_python = true             # 嵌入 Python
python_static = false           # 静态链接 Python

# 优化选项
lto = false                     # 链接时优化
strip = true                    # 剥离符号

# 调试选项
debug = false                   # 调试模式
keep_c_source = false           # 保留 C/C++ 源码
```

---

## 项目结构

```
sikuwa/
├── __init__.py              # 包初始化
├── __main__.py              # 入口点
├── cli.py                   # 命令行接口
├── config.py                # 配置管理
├── builder.py               # 构建器核心
├── compiler.py              # Native 编译器
├── parser.py                # 代码解析器
├── log.py                   # 日志系统
├── i18n.py                  # 国际化支持
├── nuitka_loader.py         # Nuitka 加载器
├── cpp_cache/               # C++ 缓存扩展
│   ├── smart_cache.cpp
│   ├── smart_cache.h
│   └── pysmartcache.cpp
├── incremental/             # 增量编译模块
│   ├── core.py              # 核心实现
│   ├── analyzer.py          # 代码分析器
│   ├── smart_cache.py       # 智能缓存
│   └── compiler_integration.py
└── i18n/                    # 国际化资源
    └── locales/
        └── en_US/
```

---

## Documentation

### 在线文档

- [官方文档](https://www.sanrol-cloud.top)
- [API 参考](https://www.sanrol-cloud.top/api)
- [示例项目](https://github.com/FORGE24/Sikuwa/tree/main/examples)

### 本地文档

```bash
# 生成文档
cd docs
make html
```

---

## Contributing

欢迎贡献代码、报告问题或提出改进建议。

### 贡献流程

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 创建 Pull Request

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/FORGE24/Sikuwa.git
cd Sikuwa

# 创建虚拟环境
python -m venv .venv
.venv\Scripts\activate  # Windows
source .venv/bin/activate  # Linux/macOS

# 安装开发依赖
pip install -e ".[dev]"

# 运行测试
pytest tests/
```

### 代码规范

- 遵循 PEP 8 代码风格
- 使用类型注解
- 编写单元测试
- 更新相关文档

### 提交规范

提交信息格式：

```
<type>(<scope>): <subject>

<body>

<footer>
```

类型 (type)：
- `feat`: 新功能
- `fix`: 修复 Bug
- `docs`: 文档更新
- `style`: 代码格式
- `refactor`: 重构
- `test`: 测试相关
- `chore`: 构建/工具

---

## 常见问题

<details>
<summary><b>Q: 构建时提示找不到 Nuitka？</b></summary>

A: 运行 `sikuwa doctor` 检查环境，确保已安装 Nuitka：

```bash
pip install nuitka
```
</details>

<details>
<summary><b>Q: Native 模式需要哪些编译器？</b></summary>

A: Native 模式需要 GCC/G++ 编译器。Windows 用户推荐安装 MinGW-w64 或使用 MSYS2。
</details>

<details>
<summary><b>Q: 如何减少构建产物体积？</b></summary>

A: 
1. 使用 `onefile = true` 生成单文件
2. 启用 `lto = true` 链接时优化
3. 使用 `nofollow_imports` 排除不需要的模块
</details>

<details>
<summary><b>Q: 支持哪些 Python 版本？</b></summary>

A: 支持 Python 3.7 及以上版本。推荐使用 Python 3.10+。
</details>

---

## 更新日志

### v1.3.0 (2026-01-31)

**新特性**
- 新增 Native 编译模式
- 增量编译系统
- C++ 智能缓存扩展
- 国际化支持

**改进**
- 优化构建性能
- 改进日志系统
- 更完善的错误处理

**修复**
- 修复 Windows 路径处理问题
- 修复大型项目编译超时问题

### v1.2.0

- 初始公开发布
- 基础 Nuitka 构建支持
- 命令行界面

查看完整更新日志：[CHANGELOG.md](CHANGELOG.md)

---

## 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

---

## 致谢

- [Nuitka](https://nuitka.net/) - Python 编译器
- [Click](https://click.palletsprojects.com/) - 命令行框架
- [pybind11](https://github.com/pybind/pybind11) - C++/Python 绑定

---

## 联系方式

- GitHub Issues: [提交问题](https://github.com/FORGE24/Sikuwa/issues)
- 官方网站: [https://www.sanrol-cloud.top](https://www.sanrol-cloud.top)

---

<p align="center">
  <sub>Made with dedication by Sikuwa Team</sub>
</p>
