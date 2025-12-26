# sikuwa/builder.py
"""
Sikuwa Builder - Ultra-detailed log tracking
支持两种编译模式：
1. Nuitka 模式：传统 Python → 机器码编译
2. Native 模式：Python → C/C++ → GCC/G++ → dll/so + exe
"""

import subprocess
import shutil
import json
import sys
import queue
from pathlib import Path
from typing import Optional, List, Dict, Set, Tuple, Any
import traceback
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor, as_completed

# Try to import smart cache extension
_use_cache = False
try:
    import sys
    import os
    from sikuwa import cpp_cache
    _use_cache = True
except ImportError as e:
    print(f"Cache extension import error: {e}")

from sikuwa.config import BuildConfig
from sikuwa.log import get_logger, PerfTimer, LogLevel
from sikuwa.i18n import _

# 尝试导入原生编译器
_native_compiler_available = False
try:
    from sikuwa.compiler import NativeCompiler, CompilerConfig, native_build, detect_compiler
    _native_compiler_available = True
except ImportError as e:
    print(f"Native compiler import error: {e}")


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
        - 初始化构建缓存系统（如果可用）。
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
        self.logger.info_operation(_("初始化 Sikuwa 构建器"))
        self.logger.info_operation("=" * 70)
        
        self.logger.debug_config(f"{_("项目名称")}: {config.project_name}")
        self.logger.debug_config(f"{_("入口文件")}: {config.main_script}")
        self.logger.debug_config(f"{_("源目录")}: {config.src_dir}")
        self.logger.debug_config(f"{_("输出目录")}: {config.output_dir}")
        self.logger.debug_config(f"{_("构建目录")}: {config.build_dir}")
        self.logger.debug_config(f"{_("目标平台")}: {config.platforms}")
        self.logger.debug_config(f"{_("详细模式")}: {verbose}")
        
        # 设置目录
        with PerfTimer(_("设置构建目录"), self.logger):
            self._setup_directories()
        
        # Initialize build cache
        self.build_cache = None
        if _use_cache:
            try:
                cache_dir = self.build_dir / ".smart_cache"
                self.build_cache = cpp_cache.build_cache_new(str(cache_dir))
                self.logger.debug_config(f"构建缓存: {_("已启用")} - {cache_dir}")
            except Exception as e:
                self.logger.debug_config(f"构建缓存: {_("初始化失败")} - {e}")
                self.build_cache = None
        else:
            self.logger.debug_config(f"构建缓存: {_("未启用")}")
        
        self.logger.info_operation(_("构建器初始化完成") + "\n")
    
    def _setup_directories(self):
        """Setup build directories"""
        self.logger.trace_flow(">>> _setup_directories")
        
        self.output_dir = Path(self.config.output_dir)
        self.build_dir = Path(self.config.build_dir)
        self.logs_dir = Path("sikuwa_logs")
        
        directories = [
            (_("输出目录"), self.output_dir),
            (_("构建目录"), self.build_dir),
            (_("日志目录"), self.logs_dir)
        ]
        
        for name, directory in directories:
            self.logger.trace_io(f"{_("检查")} {name}: {directory}")
            if not directory.exists():
                self.logger.trace_io(f"  {_("创建")} {name}: {directory}")
                directory.mkdir(parents=True, exist_ok=True)
                self.logger.debug_detail(f"  [OK] {name} {_("创建成功")}")
            else:
                self.logger.trace_io(f"  {name} {_("已存在")}")
        
        self.logger.trace_flow("<<< _setup_directories")
    
    def build(self, platform: Optional[str] = None, force: bool = False):
        """Execute complete build process"""
        self.logger.info_operation("\n" + "=" * 70)
        self.logger.info_operation(f"{_('开始构建')}: {self.config.project_name}")
        self.logger.info_operation("=" * 70)
        
        # 检查编译模式
        compiler_mode = getattr(self.config, 'compiler_mode', 'nuitka')
        self.logger.info_operation(f"{_('编译模式')}: {compiler_mode.upper()}")
        
        if compiler_mode == 'native':
            self._build_native(platform, force)
        else:
            self._build_nuitka(platform, force)
    
    def _build_native(self, platform: Optional[str] = None, force: bool = False):
        """使用原生编译器构建 (Python → C/C++ → GCC/G++ → dll/so + exe)"""
        if not _native_compiler_available:
            raise RuntimeError(
                "原生编译器模块不可用，请检查 sikuwa/compiler.py 是否存在"
            )
        
        with PerfTimer(_("原生编译流程"), self.logger):
            try:
                # Step 1: 验证环境
                self.logger.info_operation("\n[1/4] " + _("验证构建环境") + "...")
                with PerfTimer(_("验证环境"), self.logger):
                    self._validate_native_environment()
                self.logger.info_operation("[OK] " + _("环境验证通过"))
                
                # Step 2: 执行原生编译
                self.logger.info_operation("\n[2/4] " + _("执行原生编译") + "...")
                platforms = [platform] if platform else self.config.platforms
                
                for plat in platforms:
                    self.logger.info_operation(f"\n--- {_('构建平台')}: {plat} ---")
                    with PerfTimer(f"{_('原生编译')} {plat}", self.logger):
                        self._execute_native_build(plat, force)
                
                self.logger.info_operation("[OK] " + _("编译完成"))
                
                # Step 3: 复制资源
                self.logger.info_operation("\n[3/4] " + _("复制资源文件") + "...")
                with PerfTimer(_("复制资源"), self.logger):
                    self._copy_resources_native()
                self.logger.info_operation("[OK] " + _("资源复制完成"))
                
                # Step 4: 生成清单
                self.logger.info_operation("\n[4/4] " + _("生成构建清单") + "...")
                with PerfTimer(_("生成清单"), self.logger):
                    self._generate_manifest()
                self.logger.info_operation("[OK] " + _("清单生成完成"))
                
                # 构建成功
                self.logger.info_operation("\n" + "=" * 70)
                self.logger.info_operation(_("原生编译成功完成!"))
                self.logger.info_operation("=" * 70)
                self.logger.info_operation(f"{_('输出目录')}: {self.output_dir.absolute()}")
                self.logger.info_operation(_("生成文件") + ":")
                self.logger.info_operation(f"  - {self.config.project_name}.dll/so ({_('通用动态链接库')})")
                self.logger.info_operation(f"  - {self.config.project_name}.exe ({_('可执行文件')})")
                self.logger.info_operation("")
                
            except Exception as e:
                self.logger.error_minimal(f"\n[FAIL] " + _("原生编译失败") + f": {e}")
                self.logger.debug_detail(f"完整异常堆栈:\n{traceback.format_exc()}")
                raise
    
    def _validate_native_environment(self):
        """验证原生编译环境"""
        self.logger.trace_flow(">>> _validate_native_environment")
        
        # 检查 Python 版本
        python_version = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
        self.logger.debug_detail(f"Python {_('版本')}: {python_version}")
        
        # 检测 C/C++ 编译器
        try:
            cc, cxx = detect_compiler()
            self.logger.debug_detail(f"C {_('编译器')}: {cc}")
            self.logger.debug_detail(f"C++ {_('编译器')}: {cxx}")
        except RuntimeError as e:
            self.logger.error_dependency(f"[FAIL] {e}")
            raise
        
        # 检查 Cython (可选)
        try:
            import Cython
            self.logger.debug_detail(f"Cython {_('版本')}: {Cython.__version__}")
        except ImportError:
            self.logger.warn_minor(f"Cython {_('未安装')}, {_('将使用内置转换器')}")
        
        # 检查入口文件
        main_file = Path(self.config.src_dir) / self.config.main_script
        if not main_file.exists():
            raise FileNotFoundError(f"{_('入口文件不存在')}: {main_file}")
        
        self.logger.trace_flow("<<< _validate_native_environment")
    
    def _execute_native_build(self, platform: str, force: bool):
        """执行原生编译"""
        # 构建编译器配置
        native_opts = self.config.native_options
        compiler_config = CompilerConfig(
            mode=native_opts.mode,
            cc=native_opts.cc,
            cxx=native_opts.cxx,
            c_flags=native_opts.c_flags,
            cxx_flags=native_opts.cxx_flags,
            link_flags=native_opts.link_flags,
            output_dll=native_opts.output_dll,
            output_exe=native_opts.output_exe,
            output_static=native_opts.output_static,
            embed_python=native_opts.embed_python,
            python_static=native_opts.python_static,
            lto=native_opts.lto,
            strip=native_opts.strip,
            debug=native_opts.debug,
            keep_c_source=native_opts.keep_c_source,
        )
        
        # 执行编译
        results = native_build(
            project_name=self.config.project_name,
            src_dir=self.config.src_dir,
            main_script=self.config.main_script,
            output_dir=str(self.output_dir),
            platform=platform,
            compiler_config=compiler_config,
            verbose=self.verbose
        )
        
        self.logger.info_operation(f"[OK] {_('生成文件')}:")
        for file_type, file_path in results.items():
            self.logger.info_operation(f"  - {file_type}: {file_path.name}")
    
    def _copy_resources_native(self):
        """复制资源文件到原生编译输出目录"""
        if not self.config.resources:
            self.logger.debug_detail(_("没有需要复制的资源文件"))
            return
        
        for platform in self.config.platforms:
            platform_dir = self.output_dir / f"native-{platform}"
            
            if not platform_dir.exists():
                self.logger.warn_minor(f"[WARN] {_('平台目录不存在')}: {platform_dir}")
                continue
            
            for resource in self.config.resources:
                src = Path(resource)
                if src.exists():
                    if src.is_dir():
                        dest = platform_dir / src.name
                        shutil.copytree(src, dest, dirs_exist_ok=True)
                    else:
                        shutil.copy2(src, platform_dir / src.name)
                    self.logger.trace_io(f"  {_('复制')}: {src.name}")
    
    def _build_nuitka(self, platform: Optional[str] = None, force: bool = False):
        """使用 Nuitka 构建 (原有逻辑)"""
        with PerfTimer(_("完整构建流程"), self.logger):
            try:
                # Step 1: Validate environment
                self.logger.info_operation("\n[1/5] " + _("验证构建环境") + "...")
                with PerfTimer(_("验证环境"), self.logger):
                    self._validate_environment()
                self.logger.info_operation("[OK] " + _("环境验证通过"))
                
                # Step 2: Prepare source code
                self.logger.info_operation("\n[2/5] " + _("准备源代码") + "...")
                with PerfTimer(_("准备源代码"), self.logger):
                    self._prepare_source()
                self.logger.info_operation("[OK] " + _("源代码准备完成"))
                
                # Step 3: Execute compilation
                self.logger.info_operation("\n[3/5] " + _("执行编译") + "...")
                if platform:
                    self.logger.info_operation(f"   {_('目标平台')}: {platform}")
                    with PerfTimer(f"{_('编译')} {platform}", self.logger):
                        self._build_single_platform(platform, force)
                else:
                    self.logger.info_operation(f"   {_('目标平台')}: {', '.join(self.config.platforms)}")
                    with PerfTimer(_("编译所有平台"), self.logger):
                        self._build_all_platforms(force)
                self.logger.info_operation("[OK] " + _("编译完成"))
                
                # Step 4: Copy resources
                self.logger.info_operation("\n[4/5] " + _("复制资源文件") + "...")
                with PerfTimer(_("复制资源"), self.logger):
                    self._copy_resources()
                self.logger.info_operation("[OK] " + _("资源复制完成"))
                
                # Step 5: Generate manifest
                self.logger.info_operation("\n[5/5] " + _("生成构建清单") + "...")
                with PerfTimer(_("生成清单"), self.logger):
                    self._generate_manifest()
                self.logger.info_operation("[OK] " + _("清单生成完成"))
                
                # Build successful
                self.logger.info_operation("\n" + "=" * 70)
                self.logger.info_operation(_("构建成功完成!") + "")
                self.logger.info_operation("=" * 70)
                self.logger.info_operation(f"{_('输出目录')}: {self.output_dir.absolute()}")
                self.logger.info_operation("")
                
            except Exception as e:
                self.logger.error_minimal(f"\n[FAIL] " + _("构建失败") + f": {e}")
                self.logger.debug_detail(f"完整异常堆栈:\n{traceback.format_exc()}")
                raise
    
    def _validate_environment(self):
        """Validate build environment"""
        self.logger.trace_flow(">>> _validate_environment")
        
        # Check Python version
        python_version = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
        self.logger.debug_detail(f"Python {_("版本")}: {python_version}")
        self.logger.debug_config(f"Python {_("路径")}: {sys.executable}")
        
        # Check Nuitka
        self.logger.debug_detail(_("检查 Nuitka 安装") + "...")
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
                self.logger.debug_detail(f"[OK] Nuitka {_("版本")}: {nuitka_version}")
            else:
                self.logger.error_dependency(f"[FAIL] " + _("Nuitka 检查失败"))
                self.logger.debug_detail(f"stderr: {result.stderr}")
                raise RuntimeError(_("Nuitka 未正确安装"))
                
        except FileNotFoundError:
            self.logger.error_dependency("[FAIL] " + _("Nuitka 未安装"))
            raise RuntimeError(_("请先安装 Nuitka: pip install nuitka"))
        except subprocess.TimeoutExpired:
            self.logger.warn_minor("[WARN] " + _("Nuitka 版本检查超时"))
        
        # Check entry file
        main_file = Path(self.config.src_dir) / self.config.main_script
        self.logger.trace_io(f"{_("检查入口文件")}: {main_file}")
        
        if not main_file.exists():
            self.logger.error_minimal(f"[FAIL] " + _("入口文件不存在") + f": {main_file}")
            raise FileNotFoundError(f"{_("入口文件不存在")}: {main_file}")
        
        self.logger.debug_detail(f"[OK] " + _("入口文件存在") + f": {main_file}")
        self.logger.trace_flow("<<< _validate_environment")
    
    def _prepare_source(self):
        """Prepare source code"""
        self.logger.trace_flow(">>> _prepare_source")
        
        src_dir = Path(self.config.src_dir)
        self.logger.debug_detail(f"{_("源代码目录")}: {src_dir}")
        
        if src_dir.exists():
            # Count source files
            py_files = list(src_dir.rglob("*.py"))
            self.logger.debug_detail(f"{_("发现")} {len(py_files)} {_("个 Python 文件")}")
            
            if self.verbose:
                for py_file in py_files:
                    self.logger.trace_io(f"  - {py_file.relative_to(src_dir)}")
        
        self.logger.trace_flow("<<< _prepare_source")
    
    def _build_all_platforms(self, force: bool):
        """Build all platforms"""
        self.logger.trace_flow(">>> _build_all_platforms")
        
        for platform in self.config.platforms:
            self.logger.info_operation(f"\n--- {_("构建平台")}: {platform} ---")
            with PerfTimer(f"{_("构建")} {platform}", self.logger):
                self._build_single_platform(platform, force)
        
        self.logger.trace_flow("<<< _build_all_platforms")
    
    def _build_single_platform(self, platform: str, force: bool):
        """Build single platform"""
        self.logger.trace_flow(f">>> _build_single_platform: {platform}")
        
        with PerfTimer(f"{_("构建")} {platform}", self.logger):
            try:
                # Build command
                self.logger.debug_detail(f"{_("准备构建命令")}: {platform}")
                cmd = self._build_nuitka_command(platform)
                
                if self.verbose:
                    self.logger.debug_detail(f"Nuitka {_("命令")}:")
                    for i, arg in enumerate(cmd):
                        self.logger.debug_detail(f"  [{i}] {arg}")
                
                # Check if rebuild is needed
                target = f"{self.config.project_name}-{platform}"
                command_str = ' '.join(cmd)
                
                # Generate source code hash
                import hashlib
                src_hash = hashlib.sha256()
                src_dir = Path(self.config.src_dir)
                
                # Add all Python files to the hash
                dependencies = []
                for py_file in sorted(src_dir.rglob("*.py")):
                    if py_file.is_file():
                        file_content = py_file.read_bytes()
                        src_hash.update(file_content)
                        src_hash.update(str(py_file.relative_to(src_dir)).encode())
                        dependencies.append(str(py_file.relative_to(src_dir)))
                
                needs_rebuild = True
                
                if self.build_cache and not force:
                    try:
                        needs_rebuild = cpp_cache.build_cache_needs_rebuild(self.build_cache, target, command_str, src_hash.hexdigest())
                    except Exception as e:
                        self.logger.debug_detail(f"构建缓存检查失败: {e}")
                        needs_rebuild = True
                
                if needs_rebuild:
                    # Execute compilation
                    self.logger.info_operation(f"{_("开始编译")} {platform}...")
                    self._execute_nuitka(cmd, platform)
                    
                    # Cache the result
                    if self.build_cache:
                        try:
                            cpp_cache.build_cache_cache_build_result(self.build_cache, target, command_str, src_hash.hexdigest(), "success")
                            self.logger.debug_detail(f"构建结果已缓存: {target}")
                        except Exception as e:
                            self.logger.debug_detail(f"构建结果缓存失败: {e}")
                else:
                    self.logger.info_operation(f"[SKIP] {platform} {_("构建已缓存，跳过编译")}")
                    return  # Skip the rest of the build process
                
                self.logger.info_operation(f"[OK] {platform} {_("编译完成")}")
                
            except Exception as e:
                self.logger.error_minimal(f"[FAIL] {platform} {_("编译失败")}: {e}")
                self.logger.debug_detail(f"{_("异常详情")}:\n{traceback.format_exc()}")
                raise
        
        self.logger.trace_flow(f"<<< _build_single_platform: {platform}")
    
    def _build_nuitka_command(self, platform: str) -> list:
        """Build Nuitka command"""
        self.logger.trace_flow(f">>> _build_nuitka_command: {platform}")
        
        cmd = [sys.executable, "-m", "nuitka"]
        
        # Basic options
        if self.config.nuitka_options.standalone:
            cmd.append("--standalone")
            self.logger.trace_state(_("添加选项") + ": --standalone")
        
        if self.config.nuitka_options.onefile:
            cmd.append("--onefile")
            self.logger.trace_state(_("添加选项") + ": --onefile")
        
        if self.config.nuitka_options.follow_imports:
            cmd.append("--follow-imports")
            self.logger.trace_state(_("添加选项") + ": --follow-imports")
        
        if self.config.nuitka_options.show_progress:
            cmd.append("--show-progress")
            self.logger.trace_state(_("添加选项") + ": --show-progress")
        
        if self.config.nuitka_options.enable_console is False:
            if platform == "windows":
                cmd.append("--disable-console")
                self.logger.trace_state(_("添加选项") + ": --disable-console")
            elif platform in ["linux", "macos"]:
                self.logger.debug_detail("Linux/macOS " + _("不支持") + " --disable-console")
        
        # Output directory
        output_dir = self.output_dir / f"{self.config.project_name}-{platform}"
        cmd.append(f"--output-dir={output_dir}")
        self.logger.trace_state(f"{_("输出目录")}: {output_dir}")
        
        # Output filename
        cmd.append(f"--output-filename={self.config.project_name}")
        self.logger.trace_state(f"{_("输出文件名")}: {self.config.project_name}")
        
        # Include data files
        if self.config.nuitka_options.include_data_files:
            for data_file in self.config.nuitka_options.include_data_files:
                cmd.append(f"--include-data-file={data_file}")
                self.logger.trace_state(f"{_("包含数据文件")}: {data_file}")
        
        # Include data directories
        if self.config.nuitka_options.include_data_dirs:
            for data_dir in self.config.nuitka_options.include_data_dirs:
                # Ensure data directory is in dictionary format
                if isinstance(data_dir, dict) and 'src' in data_dir and 'dest' in data_dir:
                    src = data_dir['src']
                    dest = data_dir['dest']
                    # Nuitka format: --include-data-dir=source_path=target_path
                    include_dir_arg = f"--include-data-dir={src}={dest}"
                    cmd.append(include_dir_arg)
                    self.logger.trace_state(f"{_("包含数据目录")}: {src} -> {dest}")
                else:
                    # Compatible with old format
                    cmd.append(f"--include-data-dir={data_dir}")
                    self.logger.trace_state(f"{_("包含数据目录")}: {data_dir}")
        
        # Extra options
        if self.config.nuitka_options.extra_args:
            for arg in self.config.nuitka_options.extra_args:
                cmd.append(arg)
                self.logger.trace_state(f"{_("额外选项")}: {arg}")
        
        # Entry file
        main_file = Path(self.config.src_dir) / self.config.main_script
        cmd.append(str(main_file))
        self.logger.trace_state(f"{_("入口文件")}: {main_file}")
        
        self.logger.debug_detail(f"{_("完整命令")}: {' '.join(cmd)}")
        self.logger.trace_flow(f"<<< _build_nuitka_command")
        
        return cmd
    
    def _execute_nuitka(self, cmd: list, platform: str):
        """Execute Nuitka compilation"""
        self.logger.trace_flow(f">>> _execute_nuitka: {platform}")
        
        # Create log file
        log_file = self.logs_dir / f"nuitka-{platform}-{datetime.now().strftime('%Y%m%d-%H%M%S')}.log"
        self.logger.debug_detail(f"Nuitka {_("日志文件")}: {log_file}")
        
        try:
            with open(log_file, 'w', encoding='utf-8') as f:
                f.write(f"Nuitka {_("构建日志")} - {platform}\n")
                f.write(f"{_("时间")}: {datetime.now()}\n")
                f.write(f"{_("命令")}: {' '.join(cmd)}\n")
                f.write("=" * 70 + "\n\n")
                
                self.logger.trace_io(_("启动 Nuitka 进程") + "...")
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
                    
                    # Output to console
                    if self.verbose:
                        self.logger.trace_io(f"[Nuitka] {line}")
                    else:
                        # Only show important information
                        if any(keyword in line.lower() for keyword in 
                               ['error', 'warning', 'complete', 'success', 'fail']):
                            self.logger.debug_detail(f"[Nuitka] {line}")
                    
                    # Output progress every 100 lines
                    if line_count % 100 == 0:
                        self.logger.trace_perf(f"{_("已处理")} {line_count} {_("行输出")}")
                
                # 等待进程结束
                return_code = process.wait()
                
                if return_code == 0:
                    self.logger.info_operation(f"[OK] Nuitka {_("编译成功")} ({_("共")} {line_count} {_("行输出")})")
                    self.logger.debug_detail(f"{_("完整日志")}: {log_file}")
                else:
                    self.logger.error_minimal(f"[FAIL] Nuitka {_("编译失败")} ({_("返回码")}: {return_code})")
                    self.logger.error_minimal(f"{_("查看日志")}: {log_file}")
                    
                    # Output collected error messages
                    if error_lines:
                        self.logger.error_minimal("\n[Nuitka] " + _("错误信息") + ":")
                        for error_line in error_lines[-20:]:  # 显示最后20条错误信息
                            self.logger.error_minimal(f"[Nuitka] {error_line}")
                    
                    raise RuntimeError(f"Nuitka {_("编译失败")} ({_("返回码")}: {return_code})")
                
        except Exception as e:
            self.logger.error_minimal(f"[FAIL] " + _("执行 Nuitka 时出错") + f": {e}")
            self.logger.debug_detail(f"{_("异常详情")}:\n{traceback.format_exc()}")
            raise
        
        self.logger.trace_flow(f"<<< _execute_nuitka")
    
    def _copy_resources(self):
        """Copy resource files"""
        self.logger.trace_flow(">>> _copy_resources")
        
        if not self.config.resources:
            self.logger.debug_detail(_("没有需要复制的资源文件"))
            self.logger.trace_flow("<<< _copy_resources")
            return
        
        self.logger.debug_detail(f"{_("准备复制")} {len(self.config.resources)} {_("个资源")}")
        
        for platform in self.config.platforms:
            platform_dir = self.output_dir / f"{self.config.project_name}-{platform}"
            
            if not platform_dir.exists():
                    self.logger.warn_minor(f"[WARN] " + _("平台目录不存在") + f": {platform_dir}")
                    continue
            
            self.logger.debug_detail(f"{_("处理平台")}: {platform}")
            
            for resource in self.config.resources:
                src = Path(resource)
                
                if not src.exists():
                    self.logger.warn_minor(f"[WARN] " + _("资源不存在") + f": {src}")
                    continue
                
                dst = platform_dir / src.name
                
                try:
                    if src.is_file():
                        self.logger.trace_io(f"{_("复制文件")}: {src} -> {dst}")
                        shutil.copy2(src, dst)
                        self.logger.debug_detail(f"  [OK] " + _("文件复制成功"))
                    elif src.is_dir():
                        self.logger.trace_io(f"{_("复制目录")}: {src} -> {dst}")
                        if dst.exists():
                            self.logger.trace_io(f"  {_("删除已存在的目录")}: {dst}")
                            shutil.rmtree(dst)
                        shutil.copytree(src, dst)
                        self.logger.debug_detail(f"  [OK] " + _("目录复制成功"))
                
                except Exception as e:
                    self.logger.error_minimal(f"[FAIL] " + _("复制资源失败") + f": {src} -> {dst}")
                    self.logger.debug_detail(f"{_("错误")}: {e}")
        
        self.logger.trace_flow("<<< _copy_resources")
    
    def _generate_manifest(self):
        """Generate build manifest"""
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
        
        self.logger.debug_detail(_("生成构建清单") + "...")
        
        # 收集输出文件信息
        for platform in self.config.platforms:
            platform_dir = self.output_dir / f"{self.config.project_name}-{platform}"
            
            if not platform_dir.exists():
                self.logger.warn_minor(f"[WARN] " + _("平台目录不存在") + f": {platform_dir}")
                continue
            
            self.logger.trace_io(f"{_("扫描平台目录")}: {platform_dir}")
            
            # 查找可执行文件
            executables = []
            if platform == "windows":
                executables = list(platform_dir.glob("*.exe"))
            else:
                # Linux/macOS 查找可执行文件
                for file in platform_dir.iterdir():
                    if file.is_file() and file.stat().st_mode & 0o111:
                        executables.append(file)
            
            self.logger.debug_detail(f"{_("发现")} {len(executables)} {_("个可执行文件")}")
            
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
            
            self.logger.info_operation(f"[OK] " + _("清单文件已生成") + f": {manifest_file}")
            
            # Output build summary
            self.logger.info_operation("\n" + _("构建摘要") + ":")
            self.logger.info_operation(f"  {_("项目")}: {manifest['project']}")
            self.logger.info_operation(f"  {_("版本")}: {manifest['version']}")
            self.logger.info_operation(f"  {_("构建时间")}: {manifest['build_time']}")
            self.logger.info_operation(f"  {_("目标平台")}: {', '.join(manifest['platforms'])}")
            self.logger.info_operation(f"  {_("输出文件数")}: {len(manifest['outputs'])}")
            
            if manifest['outputs']:
                self.logger.info_operation("\n  " + _("输出文件") + ":")
                for output in manifest['outputs']:
                    self.logger.info_operation(
                        f"    [{output['platform']}] {output['file']} "
                        f"({output['size_mb']} MB)"
                    )
            
        except Exception as e:
            self.logger.error_minimal(f"[FAIL] " + _("写入清单文件失败") + f": {e}")
            self.logger.debug_detail(f"{_("异常详情")}:\n{traceback.format_exc()}")
        
        self.logger.trace_flow("<<< _generate_manifest")
    
    def clean(self):
        """Clean build files"""
        self.logger.info_operation("\n" + "=" * 70)
        self.logger.info_operation(_("清理构建文件"))
        self.logger.info_operation("=" * 70)
        
        with PerfTimer(_("清理构建文件"), self.logger):
            directories_to_clean = [
                (_("输出目录"), self.output_dir),
                (_("构建目录"), self.build_dir)
            ]
            
            for name, directory in directories_to_clean:
                if directory.exists():
                    self.logger.debug_detail(f"{_("删除")} {name}: {directory}")
                    try:
                        shutil.rmtree(directory)
                        self.logger.info_operation(f"[OK] " + _("已删除") + f" {name}")
                    except Exception as e:
                        self.logger.error_minimal(f"[FAIL] " + _("删除") + f" {name} {_("失败")}: {e}")
                else:
                    self.logger.debug_detail(f"{name} {_("不存在，跳过")}")
        
        self.logger.info_operation("\n[OK] " + _("清理完成") + "\n")


class SikuwaBuilderFactory:
    """Builder Factory"""
    
    @staticmethod
    def create_from_config(config: BuildConfig, verbose: bool = False) -> SikuwaBuilder:
        """Create builder from config"""
        logger = get_logger("sikuwa.factory")
        logger.trace_flow(">>> create_from_config")
        logger.debug_config(f"{_("创建构建器")}: {config.project_name}")
        
        builder = SikuwaBuilder(config, verbose)
        
        logger.trace_flow("<<< create_from_config")
        return builder
    
    @staticmethod
    def create_from_file(config_file: str, verbose: bool = False) -> SikuwaBuilder:
        """Create builder from config file"""
        logger = get_logger("sikuwa.factory")
        logger.trace_flow(">>> create_from_file")
        logger.debug_config(f"{_("读取配置文件")}: {config_file}")
        
        config = BuildConfig.from_toml(config_file)
        builder = SikuwaBuilder(config, verbose)
        
        logger.trace_flow("<<< create_from_file")
        return builder


def topological_sort(build_sequence: List[Dict[str, Any]], dependencies: Dict[str, List[str]]) -> List[List[str]]:
    """
    拓扑排序算法 - 使用 Kahn's 算法
    
    参数:
        build_sequence: 项目列表，每个项目包含 'name' 字段
        dependencies: 依赖关系字典，键是项目名，值是依赖的项目名列表
    
    返回:
        按构建顺序分组的项目列表，同一组内的项目可以并行构建
    
    异常:
        ValueError: 当存在循环依赖时抛出
    """
    # 构建邻接表和入度字典
    adjacency_list = {}
    in_degree = {}
    project_names = {}
    
    # 初始化
    for project in build_sequence:
        name = project['name']
        project_names[name] = project
        adjacency_list[name] = []
        in_degree[name] = 0
    
    # 构建依赖关系
    for project_name, deps in dependencies.items():
        if project_name not in project_names:
            continue
            
        for dep in deps:
            if dep in project_names:
                adjacency_list[dep].append(project_name)
                in_degree[project_name] += 1
    
    # 使用队列进行拓扑排序
    q = queue.Queue()
    
    # 将所有入度为0的项目加入队列
    for project_name in in_degree:
        if in_degree[project_name] == 0:
            q.put(project_name)
    
    sorted_groups = []
    visited = 0
    
    while not q.empty():
        # 当前层级的项目数量
        level_size = q.qsize()
        current_level = []
        
        # 处理当前层级的所有项目
        for _ in range(level_size):
            project_name = q.get()
            current_level.append(project_name)
            visited += 1
            
            # 减少依赖于此项目的项目的入度
            for neighbor in adjacency_list[project_name]:
                in_degree[neighbor] -= 1
                if in_degree[neighbor] == 0:
                    q.put(neighbor)
        
        sorted_groups.append(current_level)
    
    # 检查是否存在循环依赖
    if visited != len(project_names):
        # 找出循环依赖
        remaining = [name for name, degree in in_degree.items() if degree > 0]
        raise ValueError(f"检测到循环依赖: {remaining}")
    
    return sorted_groups


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


def sync_project(config: BuildConfig, verbose: bool = False):
    """同步项目依赖"""
    logger = get_logger("sikuwa.sync")
    logger.info_operation("启动依赖同步流程")
    
    try:
        # 确定虚拟环境路径
        venv_path = Path(".venv")
        venv_python = venv_path / ("Scripts\python.exe" if sys.platform == "win32" else "bin/python")
        
        # 检查虚拟环境是否存在
        if not venv_path.exists():
            logger.info_operation("创建虚拟环境...")
            subprocess.run(
                [sys.executable, "-m", "venv", ".venv"],
                check=True,
                capture_output=True if not verbose else False,
                text=True
            )
            logger.info_operation("虚拟环境创建成功")
        else:
            logger.info_operation("使用现有虚拟环境")
        
        # 升级pip
        logger.info_operation("升级pip...")
        subprocess.run(
            [str(venv_python), "-m", "pip", "install", "--upgrade", "pip"],
            check=True,
            capture_output=True if not verbose else False,
            text=True
        )
        
        # 安装依赖
        dependencies = []
        
        # 添加requirements_file中的依赖
        if config.requirements_file:
            logger.info_operation(f"安装requirements文件: {config.requirements_file}")
            subprocess.run(
                [str(venv_python), "-m", "pip", "install", "-r", config.requirements_file],
                check=True,
                capture_output=True if not verbose else False,
                text=True
            )
        
        # 添加dependencies中的依赖
        if config.dependencies:
            logger.info_operation(f"安装配置文件中的依赖 ({len(config.dependencies)} 个)")
            for dep in config.dependencies:
                logger.debug_detail(f"安装依赖: {dep}")
                dependencies.append(dep)
        
        # 批量安装依赖
        if dependencies:
            subprocess.run(
                [str(venv_python), "-m", "pip", "install"] + dependencies,
                check=True,
                capture_output=True if not verbose else False,
                text=True
            )
        
        logger.info_operation("依赖同步流程完成")
        return True
    except subprocess.CalledProcessError as e:
        logger.error_minimal(f"依赖同步失败: {e}")
        if e.stdout:
            logger.debug_detail(f"stdout: {e.stdout}")
        if e.stderr:
            logger.debug_detail(f"stderr: {e.stderr}")
        return False
    except Exception as e:
        logger.error_minimal(f"依赖同步失败: {e}")
        logger.debug_detail(f"异常堆栈:\n{traceback.format_exc()}")
        return False


def _build_single_project(project_config: Dict[str, Any], verbose: bool = False) -> bool:
    """
    构建单个项目
    
    参数:
        project_config: 项目配置字典
        verbose: 是否显示详细日志
    
    返回:
        构建是否成功
    """
    from sikuwa.config import BuildConfig
    
    # 加载项目配置
    try:
        # 从项目目录加载配置文件
        project_dir = Path(project_config.get('dir', '.'))
        config_file = project_dir / (project_config.get('config', 'sikuwa.toml'))
        
        if not config_file.exists():
            logger = get_logger("sikuwa.build_sequence")
            logger.error_minimal(f"项目 {project_config['name']} 的配置文件不存在: {config_file}")
            return False
        
        # 加载配置
        config = BuildConfig.from_toml(str(config_file))
        
        # 设置项目目录为源目录
        config.src_dir = str(project_dir)
        
        # 构建项目
        return build_project(config, verbose=verbose)
    except Exception as e:
        logger = get_logger("sikuwa.build_sequence")
        logger.error_minimal(f"构建项目 {project_config['name']} 失败: {e}")
        logger.debug_detail(f"异常堆栈:\n{traceback.format_exc()}")
        return False


def build_sequence(config: BuildConfig, verbose: bool = False) -> bool:
    """
    执行编译序列构建
    
    参数:
        config: 包含编译序列配置的BuildConfig对象
        verbose: 是否显示详细日志
    
    返回:
        构建是否成功
    """
    logger = get_logger("sikuwa.build_sequence")
    logger.info_operation("=" * 70)
    logger.info_operation("执行编译序列构建")
    logger.info_operation("=" * 70)
    
    try:
        if not config.build_sequence:
            logger.error_minimal("未配置编译序列")
            return False
        
        # 获取依赖关系
        dependencies = config.sequence_dependencies or {}
        
        # 执行拓扑排序
        logger.info_operation("分析项目依赖关系...")
        sorted_groups = topological_sort(config.build_sequence, dependencies)
        
        logger.info_operation(f"生成构建顺序: {len(sorted_groups)} 个构建阶段")
        for i, group in enumerate(sorted_groups):
            logger.info_operation(f"阶段 {i+1}: {', '.join(group)}")
        
        # 构建项目映射
        project_map = {project['name']: project for project in config.build_sequence}
        
        # 执行构建
        success = True
        total_projects = sum(len(group) for group in sorted_groups)
        completed_projects = 0
        
        for stage_num, stage in enumerate(sorted_groups, 1):
            logger.info_operation(f"\n[{stage_num}/{len(sorted_groups)}] 构建阶段开始: {', '.join(stage)}")
            
            if config.parallel_build and len(stage) > 1:
                # 并行构建
                logger.info_operation(f"并行构建 {len(stage)} 个项目...")
                
                with ThreadPoolExecutor(max_workers=min(config.max_workers, len(stage))) as executor:
                    futures = {}
                    
                    for project_name in stage:
                        project_config = project_map[project_name]
                        future = executor.submit(_build_single_project, project_config, verbose)
                        futures[future] = project_name
                    
                    # 收集结果
                    for future in as_completed(futures):
                        project_name = futures[future]
                        completed_projects += 1
                        
                        if future.result():
                            logger.info_operation(f"[{completed_projects}/{total_projects}] ✅ 项目 {project_name} 构建成功")
                        else:
                            logger.error_minimal(f"[{completed_projects}/{total_projects}] ❌ 项目 {project_name} 构建失败")
                            success = False
            else:
                # 顺序构建
                logger.info_operation(f"顺序构建 {len(stage)} 个项目...")
                
                for project_name in stage:
                    completed_projects += 1
                    project_config = project_map[project_name]
                    
                    logger.info_operation(f"[{completed_projects}/{total_projects}] 构建项目: {project_name}")
                    
                    if _build_single_project(project_config, verbose):
                        logger.info_operation(f"[{completed_projects}/{total_projects}] ✅ 项目 {project_name} 构建成功")
                    else:
                        logger.error_minimal(f"[{completed_projects}/{total_projects}] ❌ 项目 {project_name} 构建失败")
                        success = False
        
        if success:
            logger.info_operation("\n" + "=" * 70)
            logger.info_operation("✅ 编译序列构建完成")
            logger.info_operation("=" * 70)
        else:
            logger.error_minimal("\n" + "=" * 70)
            logger.error_minimal("❌ 编译序列构建失败")
            logger.error_minimal("=" * 70)
        
        return success
        
    except ValueError as e:
        logger.error_minimal(f"编译序列错误: {e}")
        return False
    except Exception as e:
        logger.error_minimal(f"编译序列构建失败: {e}")
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
