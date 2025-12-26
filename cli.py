# sikuwa/cli.py
"""
Sikuwa 命令行界面
"""

import click
import sys
from pathlib import Path
from typing import Optional

from sikuwa.config import ConfigManager, BuildConfig, create_config
from sikuwa.builder import SikuwaBuilder, build_project, clean_project, sync_project
from sikuwa.log import get_logger, LogLevel
from sikuwa.i18n import _


# 全局日志器
logger = get_logger("sikuwa.cli")


@click.group()
@click.version_option(version="1.2.0", prog_name="sikuwa")
def cli():
    """
    Sikuwa - Python 项目打包工具
    
    基于 Nuitka 的跨平台 Python 应用构建工具
    """
    pass


@cli.command()
@click.option(
    '-c', '--config',
    type=click.Path(exists=True),
    help='配置文件路径 (默认: sikuwa.toml)'
)
@click.option(
    '-p', '--platform',
    type=click.Choice(['windows', 'linux', 'macos'], case_sensitive=False),
    help='目标平台 (默认: 构建所有平台)'
)
@click.option(
    '-m', '--mode',
    type=click.Choice(['nuitka', 'native'], case_sensitive=False),
    help='编译模式: nuitka (默认) | native (Python→C/C++→GCC→dll/so+exe)'
)
@click.option(
    '-v', '--verbose',
    is_flag=True,
    help=_('详细输出模式')
)
@click.option(
    '-f', '--force',
    is_flag=True,
    help=_('强制重新构建')
)
@click.option(
    '--keep-c-source',
    is_flag=True,
    help='保留生成的 C/C++ 源码 (仅 native 模式)'
)
def build(config: Optional[str], platform: Optional[str], mode: Optional[str], 
          verbose: bool, force: bool, keep_c_source: bool):
    """
    构建项目
    
    支持两种编译模式:
    
    \b
    1. nuitka (默认): 使用 Nuitka 编译器
    2. native: Python → C/C++ → GCC/G++ → dll/so + exe
       生成通用动态链接库，不使用 Python 专用格式 (.pyd)
    
    示例:
    
        sikuwa build                    # 使用 Nuitka 构建
        
        sikuwa build -m native          # 使用原生编译器构建
        
        sikuwa build -m native -v       # 原生编译 + 详细输出
        
        sikuwa build -p windows         # 只构建 Windows 平台
        
        sikuwa build -c my_config.toml  # 使用指定配置文件
    """
    try:
        # 加载配置
        logger.info_operation("加载构建配置...")
        build_config = ConfigManager.load_config(config)
        
        # 如果命令行指定了编译模式，覆盖配置文件中的设置
        if mode:
            build_config.compiler_mode = mode
            if mode == 'native' and keep_c_source:
                build_config.native_options.keep_c_source = True
        
        # 验证配置
        logger.info_operation("验证配置...")
        build_config.validate()
        
        # 执行构建
        logger.info_operation(f"开始构建项目: {build_config.project_name}")
        logger.info_operation(f"编译模式: {build_config.compiler_mode.upper()}")
        
        success = build_project(
            config=build_config,
            platform=platform,
            verbose=verbose,
            force=force
        )
        
        if success:
            click.echo("\n[OK] 构建成功完成!", err=False)
            sys.exit(0)
        else:
            click.echo("\n[FAIL] 构建失败!", err=True)
            sys.exit(1)
            
    except FileNotFoundError as e:
        click.echo(f"[FAIL] {e}", err=True)
        click.echo("\n提示: 使用 'sikuwa init' 创建配置文件", err=True)
        sys.exit(1)
    except Exception as e:
        click.echo(f"[FAIL] 构建失败: {e}", err=True)
        if verbose:
            import traceback
            click.echo(traceback.format_exc(), err=True)
        sys.exit(1)


@cli.command()
@click.option(
    '-c', '--config',
    type=click.Path(exists=True),
    help='配置文件路径 (默认: sikuwa.toml)'
)
@click.option(
    '-v', '--verbose',
    is_flag=True,
    help=_('详细输出模式')
)
def clean(config: Optional[str], verbose: bool):
    """
    清理构建文件
    
    删除构建过程中生成的所有文件和目录
    
    示例:
    
        sikuwa clean        # 清理构建文件
        
        sikuwa clean -v     # 详细输出
    """
    try:
        # 加载配置
        logger.info_operation("加载配置...")
        build_config = ConfigManager.load_config(config)
        
        # 执行清理
        logger.info_operation("开始清理构建文件...")
        
        success = clean_project(
            config=build_config,
            verbose=verbose
        )
        
        if success:
            click.echo("\n[OK] 清理完成!", err=False)
            sys.exit(0)
        else:
            click.echo("\n[FAIL] 清理失败!", err=True)
            sys.exit(1)
            
    except FileNotFoundError as e:
        click.echo(f"[FAIL] {e}", err=True)
        sys.exit(1)
    except Exception as e:
        click.echo(f"[FAIL] 清理失败: {e}", err=True)
        if verbose:
            import traceback
            click.echo(traceback.format_exc(), err=True)
        sys.exit(1)


@cli.command()
@click.option(
    '-o', '--output',
    default='sikuwa.toml',
    help='输出配置文件名 (默认: sikuwa.toml)'
)
@click.option(
    '--force',
    is_flag=True,
    help=_('覆盖已存在的文件')
)
def init(output: str, force: bool):
    """
    初始化项目配置
    
    创建默认的 sikuwa.toml 配置文件
    
    示例:
    
        sikuwa init                     # 创建 sikuwa.toml
        
        sikuwa init -o custom.toml      # 创建自定义配置文件
        
        sikuwa init --force             # 强制覆盖已存在的文件
    """
    try:
        output_path = Path(output)
        
        # 检查文件是否已存在
        if output_path.exists() and not force:
            click.echo(f"[WARN] 配置文件已存在: {output}", err=True)
            click.echo("使用 --force 选项强制覆盖", err=True)
            sys.exit(1)
        
        # 创建配置文件
        logger.info_operation(f"创建配置文件: {output}")
        create_config(output)
        
        click.echo(f"\n[OK] 配置文件已创建: {output}")
        click.echo("\n下一步:")
        click.echo("  1. 编辑 sikuwa.toml，配置项目信息")
        click.echo("  2. 运行 'sikuwa build' 开始构建")
        
        sys.exit(0)
        
    except Exception as e:
        click.echo(f"[FAIL] 创建配置文件失败: {e}", err=True)
        sys.exit(1)


@cli.command()
@click.option(
    '-c', '--config',
    type=click.Path(exists=True),
    help='配置文件路径 (默认: sikuwa.toml)'
)
def info(config: Optional[str]):
    """
    显示项目信息
    
    显示当前项目的配置信息
    
    示例:
    
        sikuwa info                     # 显示项目信息
        
        sikuwa info -c custom.toml      # 显示指定配置文件的信息
    """
    try:
        # 加载配置
        build_config = ConfigManager.load_config(config)
        
        # 显示项目信息
        click.echo("\n" + "=" * 70)
        click.echo(f"项目信息: {build_config.project_name}")
        click.echo("=" * 70)
        
        click.echo(f"\n基础信息:")
        click.echo(f"  项目名称: {build_config.project_name}")
        click.echo(f"  版本: {build_config.version}")
        if build_config.description:
            click.echo(f"  描述: {build_config.description}")
        if build_config.author:
            click.echo(f"  作者: {build_config.author}")
        
        click.echo(f"\n构建配置:")
        click.echo(f"  入口文件: {build_config.main_script}")
        click.echo(f"  源代码目录: {build_config.src_dir}")
        click.echo(f"  输出目录: {build_config.output_dir}")
        click.echo(f"  构建目录: {build_config.build_dir}")
        click.echo(f"  目标平台: {', '.join(build_config.platforms)}")
        
        click.echo(f"\nNuitka 选项:")
        click.echo(f"  Standalone: {build_config.nuitka_options.standalone}")
        click.echo(f"  OneFile: {build_config.nuitka_options.onefile}")
        click.echo(f"  Follow Imports: {build_config.nuitka_options.follow_imports}")
        click.echo(f"  Show Progress: {build_config.nuitka_options.show_progress}")
        click.echo(f"  Enable Console: {build_config.nuitka_options.enable_console}")
        
        if build_config.nuitka_options.include_packages:
            click.echo(f"\n包含的包:")
            for pkg in build_config.nuitka_options.include_packages:
                click.echo(f"  - {pkg}")
        
        if build_config.resources:
            click.echo(f"\n资源文件:")
            for resource in build_config.resources:
                click.echo(f"  - {resource}")
        
        click.echo("\n" + "=" * 70 + "\n")
        
    except FileNotFoundError as e:
        click.echo(f"[FAIL] {e}", err=True)
        sys.exit(1)
    except Exception as e:
        click.echo(f"[FAIL] 读取配置失败: {e}", err=True)
        sys.exit(1)


@cli.command()
def version():
    """
    显示版本信息
    """
    click.echo("\nSikuwa v1.2.0")
    click.echo("Python 项目打包工具")
    click.echo("基于 Nuitka 的跨平台构建系统")
    click.echo("\nGitHub: https://github.com/FORGE24/Sikuwa/")
    click.echo("文档: https://www.sanrol-cloud.top\n")


@cli.command()
@click.option(
    '-c', '--config',
    type=click.Path(exists=True),
    help='配置文件路径 (默认: sikuwa.toml)'
)
def validate(config: Optional[str]):
    """
    验证配置文件
    
    检查配置文件是否正确
    
    示例:
    
        sikuwa validate                 # 验证默认配置文件
        
        sikuwa validate -c custom.toml  # 验证指定配置文件
    """
    try:
        # 加载配置
        logger.info_operation("加载配置文件...")
        build_config = ConfigManager.load_config(config)
        
        # 验证配置
        logger.info_operation("验证配置...")
        build_config.validate()
        
        click.echo("\n[OK] 配置文件有效!")
        
        # 显示摘要信息
        click.echo(f"\n项目: {build_config.project_name}")
        click.echo(f"版本: {build_config.version}")
        click.echo(f"入口: {build_config.main_script}")
        click.echo(f"平台: {', '.join(build_config.platforms)}\n")
        
        sys.exit(0)
        
    except FileNotFoundError as e:
        click.echo(f"[FAIL] {e}", err=True)
        sys.exit(1)
    except ValueError as e:
        click.echo(f"[FAIL] 配置验证失败: {e}", err=True)
        sys.exit(1)
    except Exception as e:
        click.echo(f"[FAIL] 验证失败: {e}", err=True)
        sys.exit(1)


@cli.command()
def doctor():
    """
    检查构建环境
    
    检查系统环境和依赖项是否满足构建要求
    """
    import subprocess
    import platform
    
    click.echo("\n" + "=" * 70)
    click.echo("Sikuwa 环境诊断")
    click.echo("=" * 70)
    
    # 检查 Python 版本
    click.echo("\n[1] Python 环境")
    python_version = sys.version_info
    click.echo(f"  版本: {python_version.major}.{python_version.minor}.{python_version.micro}")
    click.echo(f"  路径: {sys.executable}")
    
    if python_version.major < 3 or (python_version.major == 3 and python_version.minor < 7):
        click.echo("  [FAIL] Python 版本过低，需要 3.7+", err=True)
    else:
        click.echo("  [OK] Python 版本满足要求")
    
    # 检查操作系统
    click.echo("\n[2] 操作系统")
    os_name = platform.system()
    os_version = platform.version()
    click.echo(f"  系统: {os_name}")
    click.echo(f"  版本: {os_version}")
    click.echo(f"  架构: {platform.machine()}")
    
    # 检查 Nuitka
    click.echo("\n[3] Nuitka")
    try:
        result = subprocess.run(
            ["nuitka3", "--version"],
            capture_output=True,
            text=True,
            timeout=5
        )
        if result.returncode == 0:
            version = result.stdout.strip().split('\n')[0]
            click.echo(f"  [OK] 已安装: {version}")
        else:
            click.echo("  [FAIL] Nuitka 未正确安装", err=True)
    except FileNotFoundError:
        click.echo("  [FAIL] Nuitka 未安装", err=True)
        click.echo("  安装命令: pip install nuitka")
    except Exception as e:
        click.echo(f"  [WARN] 检查 Nuitka 时出错: {e}", err=True)
    
    # 检查编译器 (Windows)
    if os_name == "Windows":
        click.echo("\n[4] C 编译器 (Windows)")
        
        # 检查 MinGW
        try:
            result = subprocess.run(
                ["gcc", "--version"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if result.returncode == 0:
                version = result.stdout.strip().split('\n')[0]
                click.echo(f"  [OK] GCC 已安装: {version}")
            else:
                click.echo("  [WARN] GCC 未找到", err=True)
        except FileNotFoundError:
            click.echo("  [WARN] GCC 未安装", err=True)
            click.echo("  推荐安装 MinGW-w64 或 MSVC")
        except Exception as e:
            click.echo(f"  [WARN] 检查 GCC 时出错: {e}", err=True)
        
        # 检查 MSVC
        try:
            result = subprocess.run(
                ["cl"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if "Microsoft" in result.stderr or "Microsoft" in result.stdout:
                click.echo("  [OK] MSVC 已安装")
            else:
                click.echo("  [INFO] MSVC 未找到")
        except FileNotFoundError:
            click.echo("  [INFO] MSVC 未安装")
        except Exception as e:
            click.echo(f"  [INFO] 检查 MSVC 时出错: {e}")
    
    # 检查编译器 (Linux/macOS)
    elif os_name in ["Linux", "Darwin"]:
        click.echo(f"\n[4] C 编译器 ({os_name})")
        
        try:
            result = subprocess.run(
                ["gcc", "--version"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if result.returncode == 0:
                version = result.stdout.strip().split('\n')[0]
                click.echo(f"  [OK] GCC 已安装: {version}")
            else:
                click.echo("  [FAIL] GCC 未找到", err=True)
        except FileNotFoundError:
            click.echo("  [FAIL] GCC 未安装", err=True)
            if os_name == "Linux":
                click.echo("  安装命令: sudo apt install gcc  # Debian/Ubuntu")
                click.echo("            sudo yum install gcc  # RedHat/CentOS")
            elif os_name == "Darwin":
                click.echo("  安装命令: xcode-select --install")
        except Exception as e:
            click.echo(f"  [WARN] 检查 GCC 时出错: {e}", err=True)
    
    # 检查必需的 Python 包
    click.echo("\n[5] Python 依赖包")
    required_packages = {
        'click': 'CLI 框架',
        'tomli': 'TOML 解析器',
        'tomli_w': 'TOML 写入器',
        'nuitka': 'Python 编译器'
    }
    
    for package, description in required_packages.items():
        try:
            __import__(package)
            click.echo(f"  [OK] {package:15s} - {description}")
        except ImportError:
            click.echo(f"  [FAIL] {package:15s} - {description} (未安装)", err=True)
    
    # 检查可选包
    click.echo("\n[6] 可选依赖包")
    optional_packages = {
        'ordered_set': _('有序集合支持'),
        'zstandard': 'Zstandard 压缩',
    }
    
    for package, description in optional_packages.items():
        try:
            __import__(package)
            click.echo(f"  [OK] {package:15s} - {description}")
        except ImportError:
            click.echo(f"  [INFO] {package:15s} - {description} (未安装)")
    
    # 总结
    click.echo("\n" + "=" * 70)
    click.echo(_("诊断完成"))
    click.echo("=" * 70)
    click.echo("\n如果有 [FAIL] 项，请先解决这些问题后再进行构建。\n")


@cli.command()
@click.argument('query', required=False)
def help_cmd(query: Optional[str]):
    """
    显示帮助信息
    
    示例:
    
        sikuwa help             # 显示总体帮助
        
        sikuwa help build       # 显示 build 命令帮助
        
        sikuwa help config      # 显示配置文件帮助
    """
    if not query:
        # 显示总体帮助
        click.echo("\n" + "=" * 70)
        click.echo("Sikuwa - Python 项目打包工具")
        click.echo("=" * 70)
        
        click.echo("\n常用命令:")
        click.echo("  sikuwa init                 创建配置文件")
        click.echo("  sikuwa build                构建项目")
        click.echo("  sikuwa clean                清理构建文件")
        click.echo("  sikuwa sync                 同步项目依赖")
        click.echo("  sikuwa info                 显示项目信息")
        click.echo("  sikuwa doctor               检查构建环境")
        
        click.echo("\n获取更多帮助:")
        click.echo("  sikuwa --help               显示所有命令")
        click.echo("  sikuwa <command> --help     显示命令详细帮助")
        click.echo("  sikuwa help config          配置文件帮助")
        
        click.echo("\n快速开始:")
        click.echo("  1. sikuwa init              # 创建配置文件")
        click.echo("  2. 编辑 sikuwa.toml         # 配置项目")
        click.echo("  3. sikuwa sync              # 同步项目依赖")
        click.echo("  4. sikuwa build             # 构建项目")
        
        click.echo("\n文档: https://www.sanrol-cloud.top")
        click.echo("=" * 70 + "\n")
        
    elif query.lower() == "config":
        # 配置文件帮助
        click.echo("\n" + "=" * 70)
        click.echo("Sikuwa 配置文件说明")
        click.echo("=" * 70)
        
        click.echo("\n配置文件示例 (sikuwa.toml):\n")
        click.echo("""[sikuwa]
project_name = "my_app"
version = "1.0.0"
description = "My Application"
author = "Your Name"

main_script = "main.py"
src_dir = "."
output_dir = "dist"
build_dir = "build"

platforms = ["windows", "linux"]
resources = ["config.json", "data/"]

[sikuwa.nuitka]
standalone = true
onefile = false
follow_imports = true
show_progress = true
enable_console = true
optimize = true

include_packages = ["requests", "numpy"]
include_modules = []
include_data_files = []
include_data_dirs = []

windows_icon = "icon.ico"
windows_company_name = "My Company"
windows_product_name = "My Product"
""")
        
        click.echo("\n主要配置项:")
        click.echo("  project_name       项目名称")
        click.echo("  main_script        入口文件")
        click.echo("  platforms          目标平台 (windows/linux/macos)")
        click.echo("  standalone         独立模式")
        click.echo("  onefile            单文件模式")
        click.echo("  include_packages   包含的 Python 包")
        
        click.echo("\n详细文档: https://www.sanrol-cloud.top")
        click.echo("=" * 70 + "\n")
        
    else:
        # 显示特定命令的帮助
        ctx = click.Context(cli)
        cmd = cli.get_command(ctx, query)
        if cmd:
            click.echo(cmd.get_help(ctx))
        else:
            click.echo(f"[FAIL] 未知命令: {query}", err=True)
            click.echo("使用 'sikuwa --help' 查看所有可用命令", err=True)


@cli.command()
@click.option(
    '-c', '--config',
    type=click.Path(exists=True),
    help='配置文件路径 (默认: sikuwa.toml)'
)
@click.option(
    '--format',
    type=click.Choice(['text', 'json'], case_sensitive=False),
    default='text',
    help='输出格式 (默认: text)'
)
def show_config(config: Optional[str], format: str):
    """
    显示完整配置
    
    以易读的格式显示所有配置选项
    
    示例:
    
        sikuwa show-config              # 显示配置 (文本格式)
        
        sikuwa show-config --format json # 显示配置 (JSON 格式)
    """
    try:
        # 加载配置
        build_config = ConfigManager.load_config(config)
        
        if format == 'json':
            # JSON 格式输出
            import json
            config_dict = build_config.to_dict()
            click.echo(json.dumps(config_dict, indent=2, ensure_ascii=False))
        else:
            # 文本格式输出
            config_dict = build_config.to_dict()
            
            click.echo("\n" + "=" * 70)
            click.echo(_("完整配置"))
            click.echo("=" * 70)
            
            def print_dict(d, indent=0):
                for key, value in d.items():
                    if isinstance(value, dict):
                        click.echo("  " * indent + f"{key}:")
                        print_dict(value, indent + 1)
                    elif isinstance(value, list):
                        click.echo("  " * indent + f"{key}:")
                        for item in value:
                            click.echo("  " * (indent + 1) + f"- {item}")
                    else:
                        click.echo("  " * indent + f"{key}: {value}")
            
            print_dict(config_dict)
            click.echo("=" * 70 + "\n")
        
        sys.exit(0)
        
    except FileNotFoundError as e:
        click.echo(f"[FAIL] {e}", err=True)
        sys.exit(1)
    except Exception as e:
        click.echo(f"[FAIL] 读取配置失败: {e}", err=True)
        sys.exit(1)


@cli.command()
@click.option(
    '-c', '--config',
    type=click.Path(exists=True),
    help='配置文件路径 (默认: sikuwa.toml)'
)
@click.option(
    '-v', '--verbose',
    is_flag=True,
    help=_('详细输出模式')
)
def sync(config: Optional[str], verbose: bool):
    """
    同步项目依赖
    
    自动创建或进入虚拟环境，并安装配置文件中指定的依赖包
    
    示例:
    
        sikuwa sync                    # 同步依赖
        
        sikuwa sync -v                 # 详细输出
        
        sikuwa sync -c my_config.toml  # 使用指定配置文件
    """
    try:
        from sikuwa.builder import sync_project
        
        # 加载配置
        logger.info_operation("加载构建配置...")
        build_config = ConfigManager.load_config(config)
        
        # 验证配置
        logger.info_operation("验证配置...")
        build_config.validate()
        
        # 执行同步
        logger.info_operation(f"开始同步项目依赖: {build_config.project_name}")
        
        success = sync_project(
            config=build_config,
            verbose=verbose
        )
        
        if success:
            click.echo("\n[OK] 依赖同步成功完成!", err=False)
            sys.exit(0)
        else:
            click.echo("\n[FAIL] 依赖同步失败!", err=True)
            sys.exit(1)
            
    except FileNotFoundError as e:
        click.echo(f"[FAIL] {e}", err=True)
        click.echo("\n提示: 使用 'sikuwa init' 创建配置文件", err=True)
        sys.exit(1)
    except Exception as e:
        click.echo(f"[FAIL] 同步失败: {e}", err=True)
        if verbose:
            import traceback
            click.echo(traceback.format_exc(), err=True)
        sys.exit(1)


@cli.command()
@click.option(
    '-c', '--config',
    type=click.Path(exists=True),
    help='配置文件路径 (默认: sikuwa.toml)'
)
@click.option(
    '-v', '--verbose',
    is_flag=True,
    help='详细输出模式'
)
def build_sequence(config: Optional[str], verbose: bool):
    """
    执行编译序列构建
    
    支持按配置文件中定义的项目依赖关系进行拓扑排序，并可选择并行构建
    
    示例:
        
        sikuwa build-sequence                    # 执行编译序列构建
        
        sikuwa build-sequence -v                 # 详细输出
        
        sikuwa build-sequence -c my_config.toml  # 使用指定配置文件
    """
    try:
        from sikuwa.builder import build_sequence
        
        # 加载配置
        logger.info_operation("加载构建配置...")
        build_config = ConfigManager.load_config(config)
        
        # 验证配置
        logger.info_operation("验证配置...")
        build_config.validate()
        
        # 执行编译序列构建
        success = build_sequence(
            config=build_config,
            verbose=verbose
        )
        
        if success:
            click.echo("\n[OK] 编译序列构建成功完成!", err=False)
            sys.exit(0)
        else:
            click.echo("\n[FAIL] 编译序列构建失败!", err=True)
            sys.exit(1)
            
    except FileNotFoundError as e:
        click.echo(f"[FAIL] {e}", err=True)
        click.echo("\n提示: 使用 'sikuwa init' 创建配置文件", err=True)
        sys.exit(1)
    except Exception as e:
        click.echo(f"[FAIL] 构建失败: {e}", err=True)
        if verbose:
            import traceback
            click.echo(traceback.format_exc(), err=True)
        sys.exit(1)


def main():
    """主入口函数"""
    try:
        cli()
    except KeyboardInterrupt:
        click.echo("\n\n[WARN] 用户中断操作", err=True)
        sys.exit(130)
    except Exception as e:
        click.echo(f"\n[FAIL] 未预期的错误: {e}", err=True)
        import traceback
        click.echo(traceback.format_exc(), err=True)
        sys.exit(1)


if __name__ == '__main__':
    main()