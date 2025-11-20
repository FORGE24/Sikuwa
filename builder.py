# sikuwa/builder.py
"""
Sikuwa 构建器 - 带超详细日志追踪
"""

import subprocess
import shutil
import json
import sys
from pathlib import Path
from typing import Optional, List, Dict
import traceback
from datetime import datetime

from sikuwa.config import BuildConfig
from sikuwa.log import get_logger, PerfTimer, LogLevel


class SikuwaBuilder:
    """Sikuwa 构建器 - 带超详细日志追踪"""
    
    def __init__(self, config: BuildConfig, verbose: bool = False):
        """Sikuwa 构建器
        初始化 SikuwaBuilder 实例，配置日志并准备构建所需的目录结构。
        Parameters
        ----------
        config : BuildConfig
            构建配置对象，包含项目名称、入口脚本、源目录、输出目录、构建目录、
            目标平台列表、资源及 Nuitka 相关选项等必要信息。
        verbose : bool, optional
            是否启用详细（追踪）日志模式。为 True 时将日志级别设置为 Trace Flow，
            以输出更详尽的运行与调试信息；否则使用常规操作信息级别。默认为 False。
        行为（副作用）
        --------
        - 初始化日志系统（通过 get_logger），并记录初始化开始与配置信息（项目名、入口文件、
          源目录、输出目录、构建目录、目标平台与详细模式）。
        - 在 PerfTimer 上下文中调用 _setup_directories()，确保输出目录、构建目录与日志目录存在，
          在必要时创建这些目录，并将对应路径保存为实例属性（如 self.output_dir、self.build_dir、self.logs_dir）。
        - 记录构建器初始化完成的日志。
        Exceptions
        ----------
        - 如果目录创建或检查失败，_setup_directories() 中的异常将向上传播；调用者应根据需要捕获并处理这些异常。
        - 本构造函数不隐藏初始化期间发生的错误，会将异常暴露给外层调用者。
        Example
        -------
        创建构建器并准备好目录与日志记录：
            builder = SikuwaBuilder(config, verbose=True)
        """
        self.config = config
        self.verbose = verbose
        
        # 初始化日志系统
        log_level = LogLevel.TRACE_FLOW if verbose else LogLevel.INFO_OPERATION
        self.logger = get_logger(f"sikuwa.builder", level=log_level)
        
        self.logger.info_operation("=" * 70)
        self.logger.info_operation("初始化 Sikuwa 构建器")
        self.logger.info_operation("=" * 70)
        
        self.logger.debug_config(f"项目名称: {config.project_name}")
        self.logger.debug_config(f"入口文件: {config.main_script}")
        self.logger.debug_config(f"源目录: {config.src_dir}")
        self.logger.debug_config(f"输出目录: {config.output_dir}")
        self.logger.debug_config(f"构建目录: {config.build_dir}")
        self.logger.debug_config(f"目标平台: {config.platforms}")
        self.logger.debug_config(f"详细模式: {verbose}")
        
        # 设置目录
        with PerfTimer("设置构建目录", self.logger):
            self._setup_directories()
        
        self.logger.info_operation("构建器初始化完成\n")
    
    def _setup_directories(self):
        """设置构建目录"""
        self.logger.trace_flow(">>> _setup_directories")
        
        self.output_dir = Path(self.config.output_dir)
        self.build_dir = Path(self.config.build_dir)
        self.logs_dir = Path("sikuwa_logs")
        
        directories = [
            ("输出目录", self.output_dir),
            ("构建目录", self.build_dir),
            ("日志目录", self.logs_dir)
        ]
        
        for name, directory in directories:
            self.logger.trace_io(f"检查 {name}: {directory}")
            if not directory.exists():
                self.logger.trace_io(f"  创建 {name}: {directory}")
                directory.mkdir(parents=True, exist_ok=True)
                self.logger.debug_detail(f"  [OK] {name} 创建成功")
            else:
                self.logger.trace_io(f"  {name} 已存在")
        
        self.logger.trace_flow("<<< _setup_directories")
    
    def build(self, platform: Optional[str] = None, force: bool = False):
        """执行完整构建流程"""
        self.logger.info_operation("\n" + "=" * 70)
        self.logger.info_operation(f"开始构建: {self.config.project_name}")
        self.logger.info_operation("=" * 70)
        
        with PerfTimer("完整构建流程", self.logger):
            try:
                # 步骤 1: 验证环境
                self.logger.info_operation("\n[1/5] 验证构建环境...")
                with PerfTimer("验证环境", self.logger):
                    self._validate_environment()
                self.logger.info_operation("[OK] 环境验证通过")
                
                # 步骤 2: 准备源代码
                self.logger.info_operation("\n[2/5] 准备源代码...")
                with PerfTimer("准备源代码", self.logger):
                    self._prepare_source()
                self.logger.info_operation("[OK] 源代码准备完成")
                
                # 步骤 3: 执行编译
                self.logger.info_operation("\n[3/5] 执行编译...")
                if platform:
                    self.logger.info_operation(f"   目标平台: {platform}")
                    with PerfTimer(f"编译 {platform}", self.logger):
                        self._build_single_platform(platform, force)
                else:
                    self.logger.info_operation(f"   目标平台: {', '.join(self.config.platforms)}")
                    with PerfTimer("编译所有平台", self.logger):
                        self._build_all_platforms(force)
                self.logger.info_operation("[OK] 编译完成")
                
                # 步骤 4: 复制资源
                self.logger.info_operation("\n[4/5] 复制资源文件...")
                with PerfTimer("复制资源", self.logger):
                    self._copy_resources()
                self.logger.info_operation("[OK] 资源复制完成")
                
                # 步骤 5: 生成清单
                self.logger.info_operation("\n[5/5] 生成构建清单...")
                with PerfTimer("生成清单", self.logger):
                    self._generate_manifest()
                self.logger.info_operation("[OK] 清单生成完成")
                
                # 构建成功
                self.logger.info_operation("\n" + "=" * 70)
                self.logger.info_operation("构建成功完成!")
                self.logger.info_operation("=" * 70)
                self.logger.info_operation(f"输出目录: {self.output_dir.absolute()}")
                self.logger.info_operation("")
                
            except Exception as e:
                self.logger.error_minimal(f"\n[FAIL] 构建失败: {e}")
                self.logger.debug_detail(f"完整异常堆栈:\n{traceback.format_exc()}")
                raise
    
    def _validate_environment(self):
        """验证构建环境"""
        self.logger.trace_flow(">>> _validate_environment")
        
        # 检查 Python 版本
        python_version = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
        self.logger.debug_detail(f"Python 版本: {python_version}")
        self.logger.debug_detail(f"Python 路径: {sys.executable}")
        
        # 检查 Nuitka
        self.logger.debug_detail("检查 Nuitka 安装...")
        try:
            with PerfTimer("检查 Nuitka", self.logger):
                result = subprocess.run(
                    [sys.executable, "-m", "nuitka", "--version"],
                    capture_output=True,
                    text=True,
                    timeout=10
                )
            
            if result.returncode == 0:
                nuitka_version = result.stdout.strip()
                self.logger.debug_detail(f"[OK] Nuitka 版本: {nuitka_version}")
            else:
                self.logger.error_dependency(f"[FAIL] Nuitka 检查失败")
                self.logger.debug_detail(f"stderr: {result.stderr}")
                raise RuntimeError("Nuitka 未正确安装")
                
        except FileNotFoundError:
            self.logger.error_dependency("[FAIL] Nuitka 未安装")
            raise RuntimeError("请先安装 Nuitka: pip install nuitka")
        except subprocess.TimeoutExpired:
            self.logger.warn_minor("[WARN] Nuitka 版本检查超时")
        
        # 检查入口文件
        main_file = Path(self.config.src_dir) / self.config.main_script
        self.logger.trace_io(f"检查入口文件: {main_file}")
        
        if not main_file.exists():
            self.logger.error_minimal(f"[FAIL] 入口文件不存在: {main_file}")
            raise FileNotFoundError(f"入口文件不存在: {main_file}")
        
        self.logger.debug_detail(f"[OK] 入口文件存在: {main_file}")
        self.logger.trace_flow("<<< _validate_environment")
    
    def _prepare_source(self):
        """准备源代码"""
        self.logger.trace_flow(">>> _prepare_source")
        
        src_dir = Path(self.config.src_dir)
        self.logger.debug_detail(f"源代码目录: {src_dir}")
        
        if src_dir.exists():
            # 统计源文件
            py_files = list(src_dir.rglob("*.py"))
            self.logger.debug_detail(f"发现 {len(py_files)} 个 Python 文件")
            
            if self.verbose:
                for py_file in py_files:
                    self.logger.trace_io(f"  - {py_file.relative_to(src_dir)}")
        
        self.logger.trace_flow("<<< _prepare_source")
    
    def _build_all_platforms(self, force: bool):
        """构建所有平台"""
        self.logger.trace_flow(">>> _build_all_platforms")
        
        for platform in self.config.platforms:
            self.logger.info_operation(f"\n--- 构建平台: {platform} ---")
            with PerfTimer(f"构建 {platform}", self.logger):
                self._build_single_platform(platform, force)
        
        self.logger.trace_flow("<<< _build_all_platforms")
    
    def _build_single_platform(self, platform: str, force: bool):
        """构建单一平台"""
        self.logger.trace_flow(f">>> _build_single_platform: {platform}")
        
        with PerfTimer(f"构建 {platform}", self.logger):
            try:
                # 构建命令
                self.logger.debug_detail(f"准备构建命令: {platform}")
                cmd = self._build_nuitka_command(platform)
                
                if self.verbose:
                    self.logger.debug_detail(f"Nuitka 命令:")
                    for i, arg in enumerate(cmd):
                        self.logger.debug_detail(f"  [{i}] {arg}")
                
                # 执行编译
                self.logger.info_operation(f"开始编译 {platform}...")
                self._execute_nuitka(cmd, platform)
                
                self.logger.info_operation(f"[OK] {platform} 编译完成")
                
            except Exception as e:
                self.logger.error_minimal(f"[FAIL] {platform} 编译失败: {e}")
                self.logger.debug_detail(f"异常详情:\n{traceback.format_exc()}")
                raise
        
        self.logger.trace_flow(f"<<< _build_single_platform: {platform}")
    
    def _build_nuitka_command(self, platform: str) -> list:
        """构建 Nuitka 命令"""
        self.logger.trace_flow(f">>> _build_nuitka_command: {platform}")
        
        cmd = [sys.executable, "-m", "nuitka"]
        
        # 基础选项
        if self.config.nuitka_options.standalone:
            cmd.append("--standalone")
            self.logger.trace_state("添加选项: --standalone")
        
        if self.config.nuitka_options.onefile:
            cmd.append("--onefile")
            self.logger.trace_state("添加选项: --onefile")
        
        if self.config.nuitka_options.follow_imports:
            cmd.append("--follow-imports")
            self.logger.trace_state("添加选项: --follow-imports")
        
        if self.config.nuitka_options.show_progress:
            cmd.append("--show-progress")
            self.logger.trace_state("添加选项: --show-progress")
        
        if self.config.nuitka_options.enable_console is False:
            if platform == "windows":
                cmd.append("--disable-console")
                self.logger.trace_state("添加选项: --disable-console")
            elif platform in ["linux", "macos"]:
                self.logger.debug_detail("Linux/macOS 不支持 --disable-console")
        
        # 输出目录
        output_dir = self.output_dir / f"{self.config.project_name}-{platform}"
        cmd.append(f"--output-dir={output_dir}")
        self.logger.trace_state(f"输出目录: {output_dir}")
        
        # 输出文件名
        cmd.append(f"--output-filename={self.config.project_name}")
        self.logger.trace_state(f"输出文件名: {self.config.project_name}")
        
        # 包含数据文件
        if self.config.nuitka_options.include_data_files:
            for data_file in self.config.nuitka_options.include_data_files:
                cmd.append(f"--include-data-file={data_file}")
                self.logger.trace_state(f"包含数据文件: {data_file}")
        
        # 包含数据目录
        if self.config.nuitka_options.include_data_dirs:
            for data_dir in self.config.nuitka_options.include_data_dirs:
                # 确保数据目录是字典格式
                if isinstance(data_dir, dict) and 'src' in data_dir and 'dest' in data_dir:
                    src = data_dir['src']
                    dest = data_dir['dest']
                    # Nuitka 格式: --include-data-dir=源路径=目标路径
                    include_dir_arg = f"--include-data-dir={src}={dest}"
                    cmd.append(include_dir_arg)
                    self.logger.trace_state(f"包含数据目录: {src} -> {dest}")
                else:
                    # 兼容旧格式
                    cmd.append(f"--include-data-dir={data_dir}")
                    self.logger.trace_state(f"包含数据目录: {data_dir}")
        
        # 额外选项
        if self.config.nuitka_options.extra_args:
            for arg in self.config.nuitka_options.extra_args:
                cmd.append(arg)
                self.logger.trace_state(f"额外选项: {arg}")
        
        # 入口文件
        main_file = Path(self.config.src_dir) / self.config.main_script
        cmd.append(str(main_file))
        self.logger.trace_state(f"入口文件: {main_file}")
        
        self.logger.debug_detail(f"完整命令: {' '.join(cmd)}")
        self.logger.trace_flow(f"<<< _build_nuitka_command")
        
        return cmd
    
    def _execute_nuitka(self, cmd: list, platform: str):
        """执行 Nuitka 编译"""
        self.logger.trace_flow(f">>> _execute_nuitka: {platform}")
        
        # 创建日志文件
        log_file = self.logs_dir / f"nuitka-{platform}-{datetime.now().strftime('%Y%m%d-%H%M%S')}.log"
        self.logger.debug_detail(f"Nuitka 日志文件: {log_file}")
        
        try:
            with open(log_file, 'w', encoding='utf-8') as f:
                f.write(f"Nuitka 构建日志 - {platform}\n")
                f.write(f"时间: {datetime.now()}\n")
                f.write(f"命令: {' '.join(cmd)}\n")
                f.write("=" * 70 + "\n\n")
                
                self.logger.trace_io(f"启动 Nuitka 进程...")
                process = subprocess.Popen(
                    cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT
,
                    text=True,
                    bufsize=1,
                    universal_newlines=True
                )
                
                # 实时读取输出
                line_count = 0
                error_lines = []
                for line in process.stdout:
                    line = line.rstrip()
                    line_count += 1
                    
                    # 写入日志文件
                    f.write(line + "\n")
                    f.flush()
                    
                    # 收集错误信息
                    if 'error' in line.lower():
                        error_lines.append(line)
                    
                    # 输出到控制台
                    if self.verbose:
                        self.logger.trace_io(f"[Nuitka] {line}")
                    else:
                        # 只显示重要信息
                        if any(keyword in line.lower() for keyword in 
                               ['error', 'warning', 'complete', 'success', 'fail']):
                            self.logger.debug_detail(f"[Nuitka] {line}")
                    
                    # 每 100 行输出一次进度
                    if line_count % 100 == 0:
                        self.logger.trace_perf(f"已处理 {line_count} 行输出")
                
                # 等待进程结束
                return_code = process.wait()
                
                if return_code == 0:
                    self.logger.info_operation(f"[OK] Nuitka 编译成功 (共 {line_count} 行输出)")
                    self.logger.debug_detail(f"完整日志: {log_file}")
                else:
                    self.logger.error_minimal(f"[FAIL] Nuitka 编译失败 (返回码: {return_code})")
                    self.logger.error_minimal(f"查看日志: {log_file}")
                    
                    # 输出收集到的错误信息
                    if error_lines:
                        self.logger.error_minimal("\n[Nuitka] 错误信息:")
                        for error_line in error_lines[-20:]:  # 显示最后20条错误信息
                            self.logger.error_minimal(f"[Nuitka] {error_line}")
                    
                    raise RuntimeError(f"Nuitka 编译失败 (返回码: {return_code})")
                
        except Exception as e:
            self.logger.error_minimal(f"[FAIL] 执行 Nuitka 时出错: {e}")
            self.logger.debug_detail(f"异常详情:\n{traceback.format_exc()}")
            raise
        
        self.logger.trace_flow(f"<<< _execute_nuitka")
    
    def _copy_resources(self):
        """复制资源文件"""
        self.logger.trace_flow(">>> _copy_resources")
        
        if not self.config.resources:
            self.logger.debug_detail("没有需要复制的资源文件")
            self.logger.trace_flow("<<< _copy_resources")
            return
        
        self.logger.debug_detail(f"准备复制 {len(self.config.resources)} 个资源")
        
        for platform in self.config.platforms:
            platform_dir = self.output_dir / f"{self.config.project_name}-{platform}"
            
            if not platform_dir.exists():
                self.logger.warn_minor(f"[WARN] 平台目录不存在: {platform_dir}")
                continue
            
            self.logger.debug_detail(f"处理平台: {platform}")
            
            for resource in self.config.resources:
                src = Path(resource)
                
                if not src.exists():
                    self.logger.warn_minor(f"[WARN] 资源不存在: {src}")
                    continue
                
                dst = platform_dir / src.name
                
                try:
                    if src.is_file():
                        self.logger.trace_io(f"复制文件: {src} -> {dst}")
                        shutil.copy2(src, dst)
                        self.logger.debug_detail(f"  [OK] 文件复制成功")
                    elif src.is_dir():
                        self.logger.trace_io(f"复制目录: {src} -> {dst}")
                        if dst.exists():
                            self.logger.trace_io(f"  删除已存在的目录: {dst}")
                            shutil.rmtree(dst)
                        shutil.copytree(src, dst)
                        self.logger.debug_detail(f"  [OK] 目录复制成功")
                    
                except Exception as e:
                    self.logger.error_minimal(f"[FAIL] 复制资源失败: {src} -> {dst}")
                    self.logger.debug_detail(f"错误: {e}")
        
        self.logger.trace_flow("<<< _copy_resources")
    
    def _generate_manifest(self):
        """生成构建清单"""
        self.logger.trace_flow(">>> _generate_manifest")
        
        manifest = {
            "project": self.config.project_name,
            "version": self.config.version,
            "build_time": datetime.now().isoformat(),
            "platforms": self.config.platforms,
            "entry_point": self.config.main_script,
            "nuitka_options": {
                "standalone": self.config.nuitka_options.standalone,
                "onefile": self.config.nuitka_options.onefile,
                "follow_imports": self.config.nuitka_options.follow_imports,
                "enable_console": self.config.nuitka_options.enable_console,
            },
            "outputs": []
        }
        
        self.logger.debug_detail("生成构建清单...")
        
        # 收集输出文件信息
        for platform in self.config.platforms:
            platform_dir = self.output_dir / f"{self.config.project_name}-{platform}"
            
            if not platform_dir.exists():
                self.logger.warn_minor(f"[WARN] 平台目录不存在: {platform_dir}")
                continue
            
            self.logger.trace_io(f"扫描平台目录: {platform_dir}")
            
            # 查找可执行文件
            executables = []
            if platform == "windows":
                executables = list(platform_dir.glob("*.exe"))
            else:
                # Linux/macOS 查找可执行文件
                for file in platform_dir.iterdir():
                    if file.is_file() and file.stat().st_mode & 0o111:
                        executables.append(file)
            
            self.logger.debug_detail(f"发现 {len(executables)} 个可执行文件")
            
            for exe in executables:
                file_size = exe.stat().st_size
                self.logger.trace_io(f"  - {exe.name} ({file_size:,} bytes)")
                
                manifest["outputs"].append({
                    "platform": platform,
                    "file": exe.name,
                    "path": str(exe.relative_to(self.output_dir)),
                    "size": file_size,
                    "size_mb": round(file_size / (1024 * 1024), 2)
                })
        
        # 写入清单文件
        manifest_file = self.output_dir / "build_manifest.json"
        self.logger.debug_detail(f"写入清单文件: {manifest_file}")
        
        try:
            with open(manifest_file, 'w', encoding='utf-8') as f:
                json.dump(manifest, f, indent=2, ensure_ascii=False)
            
            self.logger.info_operation(f"[OK] 清单文件已生成: {manifest_file}")
            
            # 输出构建摘要
            self.logger.info_operation("\n构建摘要:")
            self.logger.info_operation(f"  项目: {manifest['project']}")
            self.logger.info_operation(f"  版本: {manifest['version']}")
            self.logger.info_operation(f"  构建时间: {manifest['build_time']}")
            self.logger.info_operation(f"  目标平台: {', '.join(manifest['platforms'])}")
            self.logger.info_operation(f"  输出文件数: {len(manifest['outputs'])}")
            
            if manifest['outputs']:
                self.logger.info_operation("\n  输出文件:")
                for output in manifest['outputs']:
                    self.logger.info_operation(
                        f"    [{output['platform']}] {output['file']} "
                        f"({output['size_mb']} MB)"
                    )
            
        except Exception as e:
            self.logger.error_minimal(f"[FAIL] 写入清单文件失败: {e}")
            self.logger.debug_detail(f"异常详情:\n{traceback.format_exc()}")
        
        self.logger.trace_flow("<<< _generate_manifest")
    
    def clean(self):
        """清理构建文件"""
        self.logger.info_operation("\n" + "=" * 70)
        self.logger.info_operation("清理构建文件")
        self.logger.info_operation("=" * 70)
        
        with PerfTimer("清理构建文件", self.logger):
            directories_to_clean = [
                ("输出目录", self.output_dir),
                ("构建目录", self.build_dir)
            ]
            
            for name, directory in directories_to_clean:
                if directory.exists():
                    self.logger.debug_detail(f"删除 {name}: {directory}")
                    try:
                        shutil.rmtree(directory)
                        self.logger.info_operation(f"[OK] 已删除 {name}")
                    except Exception as e:
                        self.logger.error_minimal(f"[FAIL] 删除 {name} 失败: {e}")
                else:
                    self.logger.debug_detail(f"{name} 不存在，跳过")
        
        self.logger.info_operation("\n[OK] 清理完成\n")


class SikuwaBuilderFactory:
    """构建器工厂"""
    
    @staticmethod
    def create_from_config(config: BuildConfig, verbose: bool = False) -> SikuwaBuilder:
        """从配置创建构建器"""
        logger = get_logger("sikuwa.factory")
        logger.trace_flow(">>> create_from_config")
        logger.debug_config(f"创建构建器: {config.project_name}")
        
        builder = SikuwaBuilder(config, verbose)
        
        logger.trace_flow("<<< create_from_config")
        return builder
    
    @staticmethod
    def create_from_file(config_file: str, verbose: bool = False) -> SikuwaBuilder:
        """从配置文件创建构建器"""
        logger = get_logger("sikuwa.factory")
        logger.trace_flow(">>> create_from_file")
        logger.debug_config(f"读取配置文件: {config_file}")
        
        config = BuildConfig.from_toml(config_file)
        builder = SikuwaBuilder(config, verbose)
        
        logger.trace_flow("<<< create_from_file")
        return builder


# 便捷函数
def build_project(
    config: BuildConfig,
    platform: Optional[str] = None,
    verbose: bool = False,
    force: bool = False
):
    """构建项目"""
    logger = get_logger("sikuwa.build")
    logger.info_operation("启动构建流程")
    
    try:
        builder = SikuwaBuilder(config, verbose)
        builder.build(platform, force)
        logger.info_operation("构建流程完成")
        return True
    except Exception as e:
        logger.error_minimal(f"构建流程失败: {e}")
        logger.debug_detail(f"异常堆栈:\n{traceback.format_exc()}")
        return False


def clean_project(config: BuildConfig, verbose: bool = False):
    """清理项目"""
    logger = get_logger("sikuwa.clean")
    logger.info_operation("启动清理流程")
    
    try:
        builder = SikuwaBuilder(config, verbose)
        builder.clean()
        logger.info_operation("清理流程完成")
        return True
    except Exception as e:
        logger.error_minimal(f"清理流程失败: {e}")
        logger.debug_detail(f"异常堆栈:\n{traceback.format_exc()}")
        return False


if __name__ == '__main__':
    # 测试构建器
    print("Sikuwa Builder - 测试模式")
    print("=" * 70)
    
    # 创建测试配置
    from sikuwa.config import NuitkaOptions
    
    test_config = BuildConfig(
        project_name="test_app",
        version="0.1.0",
        main_script="main.py",
        src_dir=".",
        platforms=["windows"],
        nuitka_options=NuitkaOptions(
            standalone=True,
            onefile=True,
            follow_imports=True,
            show_progress=True,
            enable_console=True
        )
    )
    
    # 创建构建器
    builder = SikuwaBuilder(test_config, verbose=True)
    print("\n构建器创建成功!")
    print(f"输出目录: {builder.output_dir}")
    print(f"构建目录: {builder.build_dir}")
    print(f"日志目录: {builder.logs_dir}")
