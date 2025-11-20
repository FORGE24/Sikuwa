# Sikuwa 工具使用文档

## 介绍

Sikuwa 是一款基于 Nuitka 的 Python 项目打包工具，专注于提供简单高效的跨平台编译解决方案。它通过配置化管理和自动化流程，将 Python 项目转换为独立可执行文件，支持 Windows、Linux 和 macOS 多平台分发。

### 核心优势
- **跨平台支持**：同时兼容 Windows、Linux 和 macOS 系统
- **灵活配置**：通过 TOML 配置文件定制编译参数，满足不同项目需求
- **双重环境检查**：自动检测系统依赖和编译环境，提前问题排查
- **双重使用模式**：支持预编译版（独立工具链）和源码版（Python 库）两种使用方式
- **详细日志与清单**：生成构建日志和输出清单，便于版本管理和分发

## 更新说明

### v1.3.0 主要特性
1. **智能缓存系统**
   - 基于C++实现的高性能LRU缓存算法
   - Python包装器接口，支持跨平台调用
   - 纯Python回退机制，确保兼容性
   - 智能缓存键生成策略，基于文件内容和构建参数
   - 与构建流程深度集成，自动管理缓存
   - 支持强制重建和缓存清理功能

2. **性能优化**
   - 首次构建约30秒，缓存命中约1.5秒
   - 大幅减少重复构建时间
   - 内存占用低，缓存管理高效

### v1.2.0 主要特性
1. **基础功能实现**
   - 完整的项目初始化与配置管理
   - 多平台编译支持（Windows/Linux/macOS）
   - 环境检查与依赖验证
   - 构建清单自动生成

2. **核心优化**
   - 完善的日志系统，支持详细模式追踪编译过程
   - 资源文件自动复制机制
   - 构建缓存与强制重建功能
   - 命令行交互体验优化

3. **兼容性提升**
   - 支持 Python 3.7+ 版本
   - 兼容最新 Nuitka 编译选项
   - 适配主流 C 编译器（MSVC/GCC/MinGW）

## 预编译版使用方法（独立工具链）

预编译版可作为独立工具链使用，无需安装 Python 环境，只需添加到系统 PATH 即可全局调用。

### 安装配置
1. 从官方渠道下载对应平台的预编译包
2. 解压到本地目录（如 `C:\sikuwa` 或 `~/sikuwa`）
3. 将解压目录添加到系统环境变量 `PATH` 中
4. 验证安装：
   ```bash
   sikuwa --version
   ```

### 命令大全

#### 1. 初始化配置
```bash
# 创建默认配置文件（sikuwa.toml）
sikuwa init

# 创建自定义配置文件
sikuwa init -o my_config.toml

# 强制覆盖已存在的配置文件
sikuwa init --force
```

#### 2. 构建项目
```bash
# 构建所有平台（默认配置）
sikuwa build

# 只构建特定平台
sikuwa build -p windows
sikuwa build -p linux
sikuwa build -p macos

# 使用详细输出模式（查看编译过程）
sikuwa build -v

# 使用指定配置文件
sikuwa build -c my_config.toml

# 强制重新构建（忽略缓存）
sikuwa build --force
```

#### 3. 查看项目信息
```bash
# 显示当前项目配置信息
sikuwa info

# 显示指定配置文件的信息
sikuwa info -c my_config.toml
```

#### 4. 环境检查
```bash
# 检查系统环境和依赖项
sikuwa doctor
```

#### 5. 清理构建文件
```bash
# 删除输出目录和构建缓存
sikuwa clean
```

#### 6. 查看帮助
```bash
# 显示总体帮助
sikuwa --help

# 显示特定命令帮助
sikuwa build --help

# 显示配置文件帮助
sikuwa help config
```

#### 7. 版本信息
```bash
# 显示版本信息
sikuwa version
```

## 源码版使用方法（Python 库）

源码版可作为 Python 库集成到其他项目中，通过 API 调用实现编译功能。

### 安装方法
```bash
# 从源码安装
pip install .

# 开发模式安装
pip install -e .
```

### 核心 API 使用示例

#### 1. 基础构建流程
```python
from sikuwa.config import ConfigManager
from sikuwa.builder import SikuwaBuilder

# 加载配置
config = ConfigManager.load_config("sikuwa.toml")

# 初始化构建器
builder = SikuwaBuilder(config, verbose=True)

# 执行构建（所有平台）
builder.build()

# 执行构建（特定平台）
builder.build(platform="windows")
```

#### 2. 自定义配置
```python
from sikuwa.config import BuildConfig, NuitkaOptions
from sikuwa.builder import SikuwaBuilder

# 创建自定义配置
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

# 执行构建
builder = SikuwaBuilder(config)
builder.build(force=True)
```

#### 3. 清理构建文件
```python
from sikuwa.config import ConfigManager
from sikuwa.builder import SikuwaBuilder

config = ConfigManager.load_config()
builder = SikuwaBuilder(config)
builder.clean()  # 清理输出目录和构建目录
```

#### 4. 生成构建清单
```python
from sikuwa.config import ConfigManager
from sikuwa.builder import SikuwaBuilder

config = ConfigManager.load_config()
builder = SikuwaBuilder(config)
builder._generate_manifest()  # 生成构建清单文件
```

## 编译指南

### 前置条件
- Python 3.7 或更高版本
- 系统编译器：
  - Windows：MinGW-w64 (8.1.0+) 或 MSVC (2019+)
  - Linux：GCC (7.3+)
  - macOS：Xcode Command Line Tools
- 依赖包：
  ```bash
  pip install nuitka click tomli tomli_w
  ```

### 编译步骤

1. **准备配置文件**
   ```bash
   # 生成默认配置
   sikuwa init
   
   # 编辑配置文件（关键配置项）
   # 项目名称、入口文件、目标平台、Nuitka 选项等
   ```

2. **检查环境**
   ```bash
   sikuwa doctor
   ```
   确保所有检查项均显示 `[OK]`，解决任何 `[FAIL]` 项

3. **执行编译**
   ```bash
   # 基础编译（所有平台）
   sikuwa build
   
   # 单平台编译
   sikuwa build -p windows
   
   # 详细模式编译（用于调试）
   sikuwa build -v
   ```

4. **查看输出**
   编译成功后，输出文件位于配置指定的 `output_dir`（默认 `dist` 目录），按平台分类存放：
   - Windows：`dist/项目名-windows/`
   - Linux：`dist/项目名-linux/`
   - macOS：`dist/项目名-macos/`

5. **验证结果**
   构建清单文件 `dist/build_manifest.json` 包含所有输出文件信息：
   - 项目名称和版本
   - 构建时间
   - 各平台输出文件路径和大小

## 自举指南

自举是指使用 Sikuwa 工具编译自身源代码，生成独立的 Sikuwa 可执行文件。

### 自举步骤

1. **获取源代码**
   ```bash
   git clone https://github.com/yourusername/sikuwa.git
   cd sikuwa
   ```

2. **准备环境**
   ```bash
   # 安装依赖
   pip install -r requirements.txt
   
   # 检查环境
   python -m sikuwa doctor
   ```

3. **配置自举参数**
   ```bash
   # 生成配置文件
   python -m sikuwa init
   
   # 编辑配置文件（关键配置）
   # 在 sikuwa.toml 中确保以下配置
   ```
   ```toml
   [sikuwa]
   project_name = "sikuwa"
   main_script = "sikuwa/__main__.py"
   version = "1.3.0"
   platforms = ["windows"]
   
   [sikuwa.nuitka]
   standalone = true
   onefile = true
   follow_imports = true
   enable_console = true
   ```

4. **执行自举编译**
   ```bash
   # 使用源码版编译自身
   python -m sikuwa build -v
   ```

5. **验证自举结果**
   ```bash
   # 进入输出目录
   cd dist/sikuwa-<当前平台>
   
   # 验证生成的可执行文件
   ./sikuwa --version  # Linux/macOS
   sikuwa.exe --version  # Windows
   ```

6. **测试自举版本**
   ```bash
   # 创建测试项目
   mkdir test_bootstrap && cd test_bootstrap
   
   # 使用自举生成的工具初始化项目
   ../dist/sikuwa-<当前平台>/sikuwa init
   
   # 创建简单入口文件
   echo 'print("Hello, Sikuwa!")' > main.py
   
   # 构建测试项目
   ../dist/sikuwa-<当前平台>/sikuwa build
   ```

若所有步骤正常执行，说明自举成功，生成的可执行文件可作为独立工具链使用，无需依赖 Python 环境。