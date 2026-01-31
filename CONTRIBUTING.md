# 贡献指南

感谢您对 Sikuwa 项目的关注。本文档将指导您如何参与项目贡献。

---

## 目录

- [行为准则](#行为准则)
- [如何贡献](#如何贡献)
- [开发环境](#开发环境)
- [代码规范](#代码规范)
- [提交规范](#提交规范)
- [Pull Request 流程](#pull-request-流程)
- [问题反馈](#问题反馈)

---

## 行为准则

参与本项目时，请遵守以下原则：

- 尊重所有贡献者
- 保持专业、友善的交流方式
- 接受建设性的批评和建议
- 专注于项目的最佳利益

---

## 如何贡献

### 贡献类型

| 类型 | 说明 |
|:---|:---|
| 报告 Bug | 通过 Issue 报告发现的问题 |
| 功能建议 | 提出新功能或改进建议 |
| 代码贡献 | 提交代码修复或新功能 |
| 文档改进 | 完善项目文档 |
| 测试用例 | 补充单元测试或集成测试 |

### 贡献流程

```
1. Fork 仓库
2. 创建分支
3. 编写代码
4. 运行测试
5. 提交更改
6. 创建 PR
7. 代码审查
8. 合并代码
```

---

## 开发环境

### 环境要求

| 组件 | 版本 |
|:---|:---|
| Python | >= 3.7 |
| pip | >= 21.0 |
| Git | >= 2.0 |

### 环境搭建

**1. Fork 并克隆仓库**

```bash
# GitHub
git clone https://github.com/<your-username>/Sikuwa.git

# Gitee
git clone https://gitee.com/<your-username>/Sikuwa.git

cd Sikuwa
```

**2. 创建虚拟环境**

```bash
# Windows
python -m venv .venv
.venv\Scripts\activate

# Linux/macOS
python3 -m venv .venv
source .venv/bin/activate
```

**3. 安装依赖**

```bash
pip install click tomli tomli-w nuitka pytest
```

**4. 验证安装**

```bash
python -m sikuwa --version
pytest tests/ -v
```

### 开发依赖

```
click>=8.0
tomli>=2.0 (Python < 3.11)
tomli-w>=1.0
nuitka>=2.0
pytest>=7.0
```

---

## 代码规范

### Python 代码风格

遵循 PEP 8 规范，并满足以下要求：

**格式化**

```bash
# 使用 black 格式化代码
black sikuwa/

# 使用 isort 排序导入
isort sikuwa/
```

**类型注解**

所有公共函数和方法必须包含类型注解：

```python
def build_project(
    config: BuildConfig,
    platform: Optional[str] = None,
    verbose: bool = False
) -> bool:
    """构建项目"""
    ...
```

**文档字符串**

使用 Google 风格的 docstring：

```python
def compile_module(source: Path, output: Path) -> CompileResult:
    """编译单个模块。
    
    Args:
        source: 源文件路径
        output: 输出文件路径
        
    Returns:
        CompileResult: 编译结果对象
        
    Raises:
        CompileError: 编译失败时抛出
    """
    ...
```

### 命名规范

| 类型 | 规范 | 示例 |
|:---|:---|:---|
| 模块 | 小写下划线 | `smart_cache.py` |
| 类 | 大驼峰 | `SikuwaBuilder` |
| 函数/方法 | 小写下划线 | `build_project` |
| 常量 | 大写下划线 | `MAX_RETRY_COUNT` |
| 私有成员 | 单下划线前缀 | `_internal_method` |

### 代码检查

提交前运行代码检查：

```bash
# 类型检查
mypy sikuwa/

# 代码风格检查
black --check sikuwa/
isort --check sikuwa/

# 运行测试
pytest tests/ -v --cov=sikuwa
```

---

## 提交规范

### 提交信息格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### 类型说明

| 类型 | 说明 |
|:---|:---|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `docs` | 文档更新 |
| `style` | 代码格式（不影响功能） |
| `refactor` | 代码重构（不是新功能或修复） |
| `perf` | 性能优化 |
| `test` | 测试相关 |
| `build` | 构建系统或外部依赖 |
| `ci` | CI 配置 |
| `chore` | 其他更改 |

### 范围说明

| 范围 | 说明 |
|:---|:---|
| `cli` | 命令行接口 |
| `builder` | 构建器 |
| `compiler` | 编译器 |
| `config` | 配置管理 |
| `cache` | 缓存系统 |
| `i18n` | 国际化 |
| `incremental` | 增量编译 |

### 示例

```
feat(compiler): add native compilation mode

- Implement Python to C/C++ translation
- Add GCC/G++ compilation support
- Support DLL/SO output format

Closes #123
```

```
fix(config): resolve TOML parsing error for nested tables

Fix the issue where nested tables in sikuwa.toml
were not parsed correctly when containing special characters.

Fixes #456
```

### 提交检查清单

- [ ] 代码通过所有测试
- [ ] 代码符合风格规范
- [ ] 添加了必要的测试用例
- [ ] 更新了相关文档
- [ ] 提交信息符合规范

---

## Pull Request 流程

### 创建 PR

1. 确保代码基于最新的 `main` 分支

```bash
git fetch upstream
git rebase upstream/main
```

2. 创建功能分支

```bash
git checkout -b feature/your-feature-name
```

3. 提交更改并推送

```bash
git add .
git commit -m "feat(scope): your commit message"
git push origin feature/your-feature-name
```

4. 在 GitHub/Gitee 上创建 Pull Request

### PR 模板

```markdown
## 变更描述

简要描述本次变更的内容。

## 变更类型

- [ ] Bug 修复
- [ ] 新功能
- [ ] 文档更新
- [ ] 代码重构
- [ ] 性能优化
- [ ] 其他

## 测试

描述如何测试这些变更。

## 检查清单

- [ ] 代码符合项目风格规范
- [ ] 自测通过
- [ ] 添加了相应的测试用例
- [ ] 更新了相关文档

## 关联 Issue

Closes #(issue number)
```

### PR 审查标准

- 代码质量符合规范
- 测试覆盖充分
- 文档完整
- 无安全隐患
- 性能影响可接受

---

## 问题反馈

### 报告 Bug

创建 Issue 时请包含以下信息：

1. **环境信息**
   - 操作系统及版本
   - Python 版本
   - Sikuwa 版本

2. **问题描述**
   - 预期行为
   - 实际行为
   - 复现步骤

3. **日志信息**
   - 错误信息
   - 相关日志输出

### Bug 报告模板

```markdown
## 环境信息

- OS: Windows 11
- Python: 3.10.5
- Sikuwa: 1.3.0

## 问题描述

### 预期行为

描述预期的行为。

### 实际行为

描述实际发生的行为。

### 复现步骤

1. 执行命令 `sikuwa build`
2. ...
3. ...

## 日志信息

```
粘贴相关日志
```

## 其他信息

任何其他相关信息。
```

### 功能建议

提出功能建议时请说明：

- 功能描述
- 使用场景
- 预期效果
- 可能的实现方案

---

## 分支管理

| 分支 | 说明 |
|:---|:---|
| `main` | 稳定版本分支 |
| `develop` | 开发分支 |
| `feature/*` | 功能开发分支 |
| `fix/*` | Bug 修复分支 |
| `release/*` | 发布准备分支 |

---

## 版本发布

### 版本号规范

遵循语义化版本 (Semantic Versioning)：

```
MAJOR.MINOR.PATCH
```

| 部分 | 说明 |
|:---|:---|
| MAJOR | 不兼容的 API 变更 |
| MINOR | 向后兼容的功能新增 |
| PATCH | 向后兼容的问题修复 |

### 发布流程

1. 更新版本号
2. 更新 CHANGELOG
3. 创建发布分支
4. 执行测试
5. 合并到 main
6. 创建 Tag
7. 发布到 PyPI

---

## 联系方式

- GitHub Issues: [提交问题](https://github.com/FORGE24/Sikuwa/issues)
- Gitee Issues: [提交问题](https://gitee.com/FORGE24/Sikuwa/issues)

---

感谢您的贡献。
