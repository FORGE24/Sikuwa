# 更新日志

本文档记录 Sikuwa 项目的所有重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

---

## [未发布]

### 计划中

- WebAssembly 编译目标支持
- 远程分布式编译
- 编译配置可视化工具

---

## [1.3.0] - 2026-01-31

### 新增

- **Native 编译模式**: 支持 Python 到 C/C++ 的转换，通过 GCC/G++ 编译为原生二进制
  - 生成通用动态链接库 (.dll/.so)
  - 不依赖 Python 专用格式 (.pyd)
  - 支持静态链接选项
  - 可保留生成的 C/C++ 源码用于审计

- **增量编译系统**: 实现"指哪编哪"的精确编译
  - 函数级粒度的变更检测
  - 依赖关系图追踪
  - 编译缓存持久化
  - 并行编译支持

- **C++ 智能缓存扩展**: 高性能缓存实现
  - 基于 pybind11 的 Python 绑定
  - LRU 缓存策略
  - 内存映射文件支持

- **国际化支持 (i18n)**
  - 内置中英文支持
  - 基于 Babel 的翻译框架
  - 可扩展的语言包机制

- **新增 CLI 命令**
  - `sikuwa doctor`: 环境诊断
  - `sikuwa validate`: 配置验证

### 变更

- 重构日志系统，支持多级别日志输出
- 优化构建流程，减少不必要的文件操作
- 改进配置文件解析，支持更复杂的嵌套结构

### 修复

- 修复 Windows 平台路径处理问题
- 修复大型项目编译时的内存溢出问题
- 修复并行编译时的竞态条件
- 修复 TOML 配置中特殊字符解析错误

### 性能

- 增量编译场景下构建速度提升 60%
- 缓存命中时跳过重复计算
- 优化依赖分析算法复杂度

---

## [1.2.0] - 2025-10-15

### 新增

- 初始公开发布
- 基于 Nuitka 的构建系统
- TOML 配置文件支持
- 跨平台构建 (Windows/Linux/macOS)
- Standalone 和 OneFile 模式
- 资源文件打包

### CLI 命令

- `sikuwa build`: 构建项目
- `sikuwa clean`: 清理构建文件
- `sikuwa init`: 初始化配置
- `sikuwa info`: 显示项目信息
- `sikuwa version`: 显示版本

### 配置选项

- 项目基本信息配置
- Nuitka 编译选项
- 平台特定配置
- 数据文件包含

---

## [1.1.0] - 2025-08-20

### 新增

- 插件系统支持
- 自定义构建钩子

### 变更

- 重构配置管理模块

### 修复

- 修复依赖检测遗漏问题

---

## [1.0.0] - 2025-06-01

### 新增

- 项目初始版本
- 基础构建功能
- 命令行界面原型

---

## 版本对比

| 版本 | 发布日期 | 主要特性 |
|:---|:---|:---|
| 1.3.0 | 2026-01-31 | Native 模式、增量编译、i18n |
| 1.2.0 | 2025-10-15 | 公开发布、完整 CLI |
| 1.1.0 | 2025-08-20 | 插件系统 |
| 1.0.0 | 2025-06-01 | 初始版本 |

---

## 迁移指南

### 从 1.2.x 升级到 1.3.x

**配置文件变更**

新增 `[sikuwa.native]` 配置节，用于 Native 编译模式：

```toml
# 新增配置
[sikuwa.native]
cc = "gcc"
cxx = "g++"
output_dll = true
output_exe = true
```

**CLI 变更**

`build` 命令新增参数：

```bash
# 新增 -m/--mode 参数
sikuwa build -m native

# 新增 --keep-c-source 参数
sikuwa build -m native --keep-c-source
```

**API 变更**

- `BuildConfig` 类新增 `compiler_mode` 属性
- `BuildConfig` 类新增 `native_options` 属性
- 新增 `NativeCompilerOptions` 数据类

---

## 链接

- [GitHub Releases](https://github.com/FORGE24/Sikuwa/releases)
- [Gitee Releases](https://gitee.com/FORGE24/Sikuwa/releases)
- [PyPI](https://pypi.org/project/sikuwa/)

[未发布]: https://github.com/FORGE24/Sikuwa/compare/v1.3.0...HEAD
[1.3.0]: https://github.com/FORGE24/Sikuwa/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/FORGE24/Sikuwa/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/FORGE24/Sikuwa/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/FORGE24/Sikuwa/releases/tag/v1.0.0
