
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
