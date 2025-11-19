# Sikuwa 工具使用文档

## 介绍

Sikuwa 是一款基于 Nuitka 的 Python 项目打包工具，专注于提供简单高效的跨平台编译解决方案。它通过配置化管理和自动化流程，将 Python 项目转换为独立可执行文件，支持 Windows、Linux 和 macOS 多平台分发。

### 核心优势
- **跨平台支持**：同时兼容 Windows、Linux 和 macOS 系统
- **灵活配置**：通过 TOML 配置文件定制编译参数，满足不同项目需求
- **双重环境检查**：自动检测系统依赖和编译环境，提前问题排查
- **双重使用模式**：支持预编译版（独立工具链）和源码版（Python 库）两种使用方式
- **详细日志与清单**：生成构建日志和输出清单，便于版本管理和分发
- **超详细日志系统**：34 级精细化日志追踪，支持彩色输出和性能监控

## 更新说明

### v0.1.0 主要特性
1. **架构优化**
   - 重构配置管理系统，支持更灵活的配置选项
   - 改进日志系统，支持 34 级精细化日志追踪
   - 优化构建流程，提升编译速度和稳定性

2. **功能增强**
   - 新增 Nuitka 动态加载器，支持打包版本和系统版本自动切换
   - 增强配置验证机制，支持多格式配置文件（TOML/YAML/JSON）
   - 改进资源文件处理，支持复杂目录结构复制
   - 新增构建清单自动生成功能

3. **用户体验改进**
   - 命令行界面优化，支持更多实用命令
   - 详细错误报告和调试信息
   - 性能计时器和函数追踪装饰器
   - 环境诊断工具（doctor 命令）

4. **兼容性提升**
   - 支持 Python 3.7+ 版本
   - 兼容最新 Nuitka 编译选项
   - 适配主流 C 编译器（MSVC/GCC/MinGW）
   - 支持 Windows 图标和元数据配置

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

# 显示完整配置（JSON格式）
sikuwa show-config --format json
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

# 详细清理模式
sikuwa clean -v
```

#### 6. 配置验证
```bash
# 验证配置文件有效性
sikuwa validate

# 验证指定配置文件
sikuwa validate -c my_config.toml
```

#### 7. 查看帮助
```bash
# 显示总体帮助
sikuwa --help

# 显示特定命令帮助
sikuwa build --help

# 显示配置文件帮助
sikuwa help config
```

#### 8. 版本信息
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

# 安装依赖
pip install nuitka click tomli tomli_w pyyaml
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
    windows_icon="app_icon.ico",
    include_packages=["requests", "click"],
    nofollow_import_to=["numpy", "pandas"]
)

config = BuildConfig(
    project_name="my_app",
    main_script="main.py",
    version="1.0.0",
    platforms=["windows", "linux"],
    nuitka_options=nuitka_options,
    resources=["config.json", "data/"]
)

# 执行构建
builder = SikuwaBuilder(config, verbose=True)
builder.build(force=True)
```

#### 3. 使用日志系统
```python
from sikuwa.log import get_logger, PerfTimer, LogLevel

# 获取日志器
logger = get_logger("my_app", level=LogLevel.TRACE_FLOW)

# 使用不同级别日志
logger.trace_io("I/O 操作追踪")
logger.debug_detail("详细调试信息")
logger.info_operation("业务操作记录")
logger.warn_minor("轻微警告")
logger.error_minimal("业务错误")

# 性能计时
with PerfTimer("关键操作", logger):
    # 执行耗时操作
    time.sleep(0.1)

# 函数追踪装饰器
@logger.trace_function
def my_function(x, y):
    return x + y

# 方法追踪装饰器
@logger.trace_method
def my_method(self, data):
    return process_data(data)
```

#### 4. 清理构建文件
```python
from sikuwa.config import ConfigManager
from sikuwa.builder import SikuwaBuilder

config = ConfigManager.load_config()
builder = SikuwaBuilder(config)
builder.clean()  # 清理输出目录和构建目录
```

#### 5. 生成构建清单
```python
from sikuwa.config import ConfigManager
from sikuwa.builder import SikuwaBuilder

config = ConfigManager.load_config()
builder = SikuwaBuilder(config)
builder._generate_manifest()  # 生成构建清单文件
```

#### 6. 配置管理
```python
from sikuwa.config import BuildConfig, NuitkaOptions, create_config

# 创建默认配置文件
create_config("custom_config.toml")

# 从文件加载配置
config = ConfigManager.load_config("custom_config.toml")

# 验证配置
errors = validate_config(config)
if errors:
    print("配置错误:", errors)

# 保存配置
config.save_to_toml("backup_config.toml")
```

## 编译指南

### 前置条件
- **Python 环境**：Python 3.7 或更高版本
- **系统编译器**：
  - Windows：MinGW-w64 (8.1.0+) 或 MSVC (2019+)
  - Linux：GCC (7.3+)
  - macOS：Xcode Command Line Tools
- **依赖包**：
  ```bash
  pip install nuitka click tomli tomli_w pyyaml
  ```

### 配置文件说明

#### 基础配置示例（sikuwa.toml）
```toml
[sikuwa]
project_name = "my_app"
version = "1.0.0"
description = "My Python Application"
author = "Your Name"

main_script = "main.py"
src_dir = "."
output_dir = "dist"
build_dir = "build"
platforms = ["windows", "linux"]

[sikuwa.nuitka]
standalone = true
onefile = true
follow_imports = true
show_progress = true
enable_console = true

include_packages = ["requests", "click"]
include_data_files = [
    "config.json",
    "data/images/icon.png"
]

windows_icon = "icon.ico"
windows_company_name = "My Company"
windows_product_name = "My Product"
```

#### 高级配置选项
```toml
[sikuwa.nuitka]
# 优化选项
optimize = true
lto = false  # 链接时优化

# 插件管理
enable_plugins = ["tk-inter", "numpy"]
disable_plugins = ["django"]

# 排除模块
nofollow_imports = ["test", "debug"]
nofollow_import_to = ["numpy", "pandas"]

# 平台特定配置
windows_file_version = "1.0.0.0"
windows_product_version = "1.0.0"
macos_app_bundle = true
macos_icon = "app_icon.icns"

# 额外参数
extra_args = [
    "--include-module=secret_module",
    "--windows-uac-admin"
]
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
   
   # 强制重新构建
   sikuwa build --force
   ```

4. **查看输出**
   编译成功后，输出文件位于配置指定的 `output_dir`（默认 `dist` 目录），按平台分类存放：
   - Windows：`dist/项目名-windows/项目名.exe`
   - Linux：`dist/项目名-linux/项目名`
   - macOS：`dist/项目名-macos/项目名`

5. **验证结果**
   构建清单文件 `dist/build_manifest.json` 包含所有输出文件信息：
   - 项目名称和版本
   - 构建时间
   - 各平台输出文件路径和大小
   - 编译选项摘要

### 高级编译技巧

#### 1. 性能优化编译
```bash
# 启用 LTO 优化
sikuwa build --extra-args="--lto=yes"

# 启用最大优化级别
sikuwa build --extra-args="--optimize=3"
```

#### 2. 资源文件处理
```toml
[sikuwa.nuitka]
include_data_files = [
    "config.json=config/",
    "data/images/=images/",
    "*.txt=documents/"
]

include_data_dirs = [
    "static/=static/",
    "templates/=templates/"
]
```

#### 3. 调试信息保留
```toml
[sikuwa.nuitka]
# 保留调试符号
debug = true
# 生成编译报告
generate_report = true
```

## 自举指南

自举是指使用 Sikuwa 工具编译自身源代码，生成独立的 Sikuwa 可执行文件。

### 自举步骤

1. **获取源代码**


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
   
   # 编辑配置文件 sikuwa.toml
   ```
   ```toml
   [sikuwa]
   project_name = "sikuwa"
   main_script = "sikuwa/__main__.py"
   version = "1.2.0"
   platforms = ["windows", "linux", "macos"]
   description = "Sikuwa Python Packager"
   author = "Sikuwa Team"
   
   [sikuwa.nuitka]
   standalone = true
   onefile = true
   follow_imports = true
   enable_console = true
   show_progress = true
   
   include_packages = [
       "click", "tomli", "tomli_w", "pyyaml"
   ]
   
   nofollow_import_to = [
       "numpy", "pandas", "matplotlib"
   ]
   
   windows_icon = "assets/icon.ico"
   windows_company_name = "Sikuwa"
   windows_product_name = "Sikuwa Packager"
   ```

4. **执行自举编译**
   ```bash
   # 使用源码版编译自身
   python -m sikuwa build -v --force
   
   # 或者使用详细模式追踪编译过程
   python -m sikuwa build --verbose --show-progress
   ```

5. **验证自举结果**
   ```bash
   # 进入输出目录
   cd dist/sikuwa-<当前平台>
   
   # 验证生成的可执行文件
   ./sikuwa --version  # Linux/macOS
   sikuwa.exe --version  # Windows
   
   # 测试功能完整性
   ./sikuwa doctor
   ./sikuwa info
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
   ../dist/sikuwa-<当前平台>/sikuwa build -v
   
   # 验证测试项目输出
   cd dist/test_project-<平台>/
   ./test_project  # 运行编译后的程序
   ```

### 自举优化技巧

#### 1. 减小可执行文件大小
```toml
[sikuwa.nuitka]
# 启用压缩
enable_compression = true
# 移除调试信息
strip = true
# 使用 UPX 压缩（如已安装）
use_upx = true
```

#### 2. 提高启动速度
```toml
[sikuwa.nuitka]
# 启用预编译缓存
enable_cache = true
# 优化导入查找
improved_recursion = true
```

#### 3. 多平台自举
```bash
# 交叉编译支持（需要相应工具链）
sikuwa build -p windows --cross-compile
sikuwa build -p linux --cross-compile  
sikuwa build -p macos --cross-compile
```

### 自举验证清单

完成自举后，请验证以下项目：

- [ ] 可执行文件能正常启动并显示版本信息
- [ ] 所有核心命令（build、clean、init、info）正常工作
- [ ] 配置文件解析和验证功能正常
- [ ] 日志系统和错误处理正常工作
- [ ] 生成的程序能在目标平台独立运行
- [ ] 编译后的文件大小在合理范围内
- [ ] 启动速度和性能符合预期

若所有验证项通过，说明自举成功，生成的可执行文件可作为独立工具链使用，无需依赖 Python 环境。
