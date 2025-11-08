# Sikuwa

Sikuwa 是一款基于 Nuitka 的 Python 项目构建工具，旨在简化 Python 代码到可执行文件的编译、打包流程，支持跨平台构建、自动化脚本执行和增量构建，帮助开发者快速分发 Python 项目。


## 功能特点

- **便捷编译**：通过配置文件 `build.ski` 定义构建规则，无需记忆复杂的 Nuitka 命令参数
- **跨平台支持**：一键构建 Windows/macOS/Linux 平台的可执行文件
- **自动化流程**：支持预构建（preBuild）和后构建（postBuild）命令，实现依赖安装、文件处理等自动化操作
- **资源管理**：自动复制静态文件、配置文件等资源到输出目录
- **增量构建**：仅在源码变更时重新编译，提升开发效率
- **版本管理**：支持构建时自动递增版本号
- **产物管理**：提供清理临时文件和打包为 ZIP 的功能


## 安装

1. 确保已安装 Python 3.7+
2. 安装依赖：
   ```bash
   pip install nuitka
   ```
3. 克隆或下载 Sikuwa 工具到本地，将工具目录添加到系统 PATH 或直接在项目中使用


## 快速开始

### 1. 项目结构（示例）

```
my_project/
├── build.ski       # 构建配置文件（必需）
├── src/            # 源码目录（可通过配置修改）
│   └── main.py     # 入口脚本（可通过配置修改）
├── static/         # 静态资源（示例）
└── config.ini      # 配置文件（示例）
```


### 2. 创建配置文件 `build.ski`

```ini
# 项目基本信息
project = "MyApp"
version = "1.0.0"
srcDir = "src"           # 源码目录
mainScript = "main.py"   # 入口脚本
outputDir = "dist"       # 输出目录
buildDir = "build"       # 临时构建目录

# 目标平台（current/windows/macos/linux）
platforms = ["current", "windows"]

# Nuitka 编译参数
nuitka {
  standalone = true      # 生成独立可执行文件
  followImports = true   # 跟踪导入依赖
  windowsIcon = "icon.ico"  # Windows 图标（可选）
  includePackages = ["requests"]  # 强制包含的包
}

# 资源文件（从源路径复制到目标路径）
resources {
  from "static/*" to "static"  # 复制 static 目录下的文件
  from "config.ini" to "."     # 复制配置文件到根目录
}

# 构建前命令（如安装依赖）
preBuild {
  commands = [
    "pip install -r requirements.txt"
  ]
}

# 构建后命令（如清理临时文件）
postBuild {
  commands = [
    "echo 构建完成！"
  ]
}
```


### 3. 执行构建命令

在项目根目录运行以下命令：

| 命令 | 说明 |
|------|------|
| `sikuwa build` | 构建当前平台的项目（自动识别运行环境） |
| `sikuwa build --platform windows` | 构建 Windows 平台的项目 |
| `sikuwa build --auto-increment` | 构建并自动递增版本号（如 `1.0.0` → `1.0.1`） |
| `sikuwa build --force` | 忽略增量缓存，强制重新构建 |
| `sikuwa clean` | 清理临时构建文件 |
| `sikuwa clean --clean-all` | 清理临时文件和输出目录（`dist`） |
| `sikuwa package` | 将构建结果打包为 ZIP |
| `sikuwa package --platform windows` | 打包指定平台的构建结果 |
| `sikuwa help` | 查看帮助信息 |


## 配置文件详解

`build.ski` 是 Sikuwa 的核心配置文件，支持以下配置项：

### 顶层配置

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `project` | 项目名称 | `MyProject` |
| `version` | 版本号 | `1.0.0` |
| `srcDir` | 源码目录路径 | `src` |
| `mainScript` | 入口脚本文件名（位于 `srcDir` 下） | `main.py` |
| `outputDir` | 构建产物输出目录 | `dist` |
| `buildDir` | 临时构建目录 | `build` |
| `platforms` | 目标构建平台列表 | `["current"]` |


### `nuitka` 块（编译参数）

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `standalone` | 是否生成独立可执行文件 | `true` |
| `followImports` | 是否跟踪导入的依赖 | `true` |
| `removeOutput` | 是否清理中间输出 | `true` |
| `showProgress` | 是否显示编译进度 | `true` |
| `includePackages` | 强制包含的包列表 | `[]` |
| `includeModules` | 强制包含的模块列表 | `[]` |
| `excludeModules` | 排除的模块列表 | `[]` |
| `windowsIcon` | Windows 平台图标路径（`ico` 文件） | `None` |
| `windowsCompany` | Windows 公司名称 | `None` |
| `windowsProduct` | Windows 产品名称 | `None` |
| `macosAppName` | macOS 应用名称 | `None` |


### `resources` 块（资源文件）

定义需要复制到输出目录的资源文件，格式：
```ini
resources {
  from "源路径" to "目标路径"  # 支持 glob 模式（如 "static/*"）
}
```


### `preBuild`/`postBuild` 块（构建前后命令）

定义构建前/后需要执行的命令，格式：
```ini
preBuild {
  commands = [
    "命令1",
    "命令2"
  ]
}
```
支持变量替换：`${PROJECT_NAME}`、`${VERSION}`、`${OUTPUT_DIR}`、`${PLATFORM}`


## 构建产物

- 构建结果默认输出到 `dist/项目名-版本-平台/` 目录
- 日志文件保存到 `sikuwa_logs/` 目录
- 打包后的 ZIP 文件位于 `dist/` 目录下


## 注意事项

- 跨平台构建需确保本地环境支持（如在 Linux 上构建 Windows 产物需安装 Wine）
- 复杂项目可能需要手动配置 `includePackages` 或 `includeModules` 以确保依赖被正确打包
- 增量构建基于源码目录的哈希值判断，修改配置文件不会触发重新构建（需使用 `--force`）
