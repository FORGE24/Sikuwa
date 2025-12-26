# sikuwa/compiler.py
"""
Sikuwa Native Compiler - Python → C/C++ → GCC/G++ → dll/so + exe
不使用 Python 专用链接库格式，生成通用动态链接库
"""

import subprocess
import shutil
import sys
import os
import tempfile
import hashlib
from pathlib import Path
from typing import Optional, List, Dict, Any, Tuple
from dataclasses import dataclass, field
from datetime import datetime
import traceback

# 兼容扁平结构和包结构的导入
try:
    from sikuwa.log import get_logger, PerfTimer, LogLevel
    from sikuwa.i18n import _
except ImportError:
    from log import get_logger, PerfTimer, LogLevel
    from i18n import _


@dataclass
class CompilerConfig:
    """编译器配置"""
    
    # 编译模式
    mode: str = "native"  # native | cython | cffi
    
    # 编译器选择
    cc: str = "gcc"       # C 编译器
    cxx: str = "g++"      # C++ 编译器
    
    # 编译选项
    c_flags: List[str] = field(default_factory=lambda: ["-O2", "-fPIC"])
    cxx_flags: List[str] = field(default_factory=lambda: ["-O2", "-fPIC", "-std=c++17"])
    link_flags: List[str] = field(default_factory=list)
    
    # 输出选项
    output_dll: bool = True      # 生成 dll/so
    output_exe: bool = True      # 生成 exe
    output_static: bool = False  # 生成静态库
    
    # 嵌入 Python
    embed_python: bool = True    # 嵌入 Python 解释器
    python_static: bool = False  # 静态链接 Python
    
    # 优化选项
    lto: bool = False            # Link Time Optimization
    strip: bool = True           # 剥离符号
    
    # 调试选项
    debug: bool = False          # 调试模式
    keep_c_source: bool = False  # 保留生成的 C/C++ 源码
    
    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        return {
            'mode': self.mode,
            'cc': self.cc,
            'cxx': self.cxx,
            'c_flags': self.c_flags,
            'cxx_flags': self.cxx_flags,
            'link_flags': self.link_flags,
            'output_dll': self.output_dll,
            'output_exe': self.output_exe,
            'output_static': self.output_static,
            'embed_python': self.embed_python,
            'python_static': self.python_static,
            'lto': self.lto,
            'strip': self.strip,
            'debug': self.debug,
            'keep_c_source': self.keep_c_source,
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'CompilerConfig':
        """从字典创建"""
        valid_fields = {'mode', 'cc', 'cxx', 'c_flags', 'cxx_flags', 'link_flags',
                        'output_dll', 'output_exe', 'output_static', 'embed_python',
                        'python_static', 'lto', 'strip', 'debug', 'keep_c_source'}
        filtered = {k: v for k, v in data.items() if k in valid_fields}
        return cls(**filtered)


class PythonInfo:
    """Python 环境信息"""
    
    def __init__(self):
        self.version = f"{sys.version_info.major}.{sys.version_info.minor}"
        self.version_full = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
        self.executable = sys.executable
        self.prefix = sys.prefix
        self.base_prefix = sys.base_prefix
        
        # 获取编译相关路径
        self._detect_paths()
    
    def _detect_paths(self):
        """检测 Python 开发路径"""
        import sysconfig
        
        self.include_dir = sysconfig.get_path('include')
        self.stdlib_dir = sysconfig.get_path('stdlib')
        
        # 获取链接库路径
        if sys.platform == 'win32':
            self.lib_dir = Path(sys.prefix) / 'libs'
            self.lib_name = f"python{sys.version_info.major}{sys.version_info.minor}"
            self.dll_name = f"python{sys.version_info.major}{sys.version_info.minor}.dll"
        else:
            self.lib_dir = Path(sysconfig.get_config_var('LIBDIR') or '/usr/lib')
            self.lib_name = sysconfig.get_config_var('LDLIBRARY') or f"python{self.version}"
            self.dll_name = f"libpython{self.version}.so"
        
        # 获取编译标志
        self.cflags = sysconfig.get_config_var('CFLAGS') or ''
        self.ldflags = sysconfig.get_config_var('LDFLAGS') or ''


class NativeCompiler:
    """
    原生编译器 - Python → C/C++ → GCC/G++ → dll/so + exe
    
    编译流程:
    1. Python 源码 → Cython → C/C++ 源码
    2. C/C++ 源码 → GCC/G++ 编译 → 目标文件 (.o)
    3. 目标文件 → 链接 → dll/so 动态链接库 + exe 可执行文件
    """
    
    def __init__(self, config: CompilerConfig, verbose: bool = False):
        self.config = config
        self.verbose = verbose
        
        # 初始化日志
        log_level = LogLevel.TRACE_FLOW if verbose else LogLevel.INFO_OPERATION
        self.logger = get_logger("sikuwa.compiler", level=log_level)
        
        # Python 环境信息
        self.python_info = PythonInfo()
        
        # 工作目录
        self.work_dir: Optional[Path] = None
        self.c_source_dir: Optional[Path] = None
        self.obj_dir: Optional[Path] = None
        self.output_dir: Optional[Path] = None
        
        self.logger.info_operation("=" * 70)
        self.logger.info_operation(_("初始化原生编译器"))
        self.logger.info_operation("=" * 70)
        self.logger.debug_config(f"Python {_('版本')}: {self.python_info.version_full}")
        self.logger.debug_config(f"C {_('编译器')}: {config.cc}")
        self.logger.debug_config(f"C++ {_('编译器')}: {config.cxx}")
        self.logger.debug_config(f"{_('编译模式')}: {config.mode}")
    
    def compile_project(
        self,
        project_name: str,
        src_dir: Path,
        main_script: str,
        output_dir: Path,
        platform: str
    ) -> Dict[str, Path]:
        """
        编译整个项目
        
        Returns:
            Dict[str, Path]: 生成的文件路径 {'dll': Path, 'exe': Path, ...}
        """
        self.logger.info_operation(f"\n{_('开始编译项目')}: {project_name}")
        
        results = {}
        
        with PerfTimer(_("完整编译流程"), self.logger):
            try:
                # Step 1: 设置工作目录
                self._setup_work_dirs(output_dir, platform)
                
                # Step 2: 收集 Python 源文件
                py_files = self._collect_python_files(src_dir)
                self.logger.info_operation(f"  {_('发现')} {len(py_files)} {_('个 Python 文件')}")
                
                # Step 3: Python → C/C++ 转换
                self.logger.info_operation(f"\n[1/4] Python → C/C++ {_('转换')}...")
                with PerfTimer("Python → C/C++", self.logger):
                    c_files = self._python_to_c(py_files, src_dir, main_script)
                self.logger.info_operation(f"  [OK] {_('生成')} {len(c_files)} {_('个 C/C++ 文件')}")
                
                # Step 4: C/C++ → 目标文件
                self.logger.info_operation(f"\n[2/4] C/C++ → {_('目标文件')}...")
                with PerfTimer("C/C++ → .o", self.logger):
                    obj_files = self._compile_c_files(c_files)
                self.logger.info_operation(f"  [OK] {_('生成')} {len(obj_files)} {_('个目标文件')}")
                
                # Step 5: 链接生成 dll/so
                if self.config.output_dll:
                    self.logger.info_operation(f"\n[3/4] {_('链接生成动态库')}...")
                    with PerfTimer(_("链接 dll/so"), self.logger):
                        dll_path = self._link_shared_library(obj_files, project_name, platform)
                    results['dll'] = dll_path
                    self.logger.info_operation(f"  [OK] {dll_path.name}")
                
                # Step 6: 链接生成 exe
                if self.config.output_exe:
                    self.logger.info_operation(f"\n[4/4] {_('链接生成可执行文件')}...")
                    with PerfTimer(_("链接 exe"), self.logger):
                        exe_path = self._link_executable(obj_files, project_name, platform)
                    results['exe'] = exe_path
                    self.logger.info_operation(f"  [OK] {exe_path.name}")
                
                # Step 7: 复制运行时依赖
                self.logger.info_operation(f"\n{_('复制运行时依赖')}...")
                with PerfTimer(_("复制依赖"), self.logger):
                    self._copy_runtime_deps(platform)
                
                # 清理临时文件
                if not self.config.keep_c_source:
                    self._cleanup()
                
                self.logger.info_operation(f"\n[OK] {_('编译完成')}!")
                return results
                
            except Exception as e:
                self.logger.error_minimal(f"[FAIL] {_('编译失败')}: {e}")
                self.logger.debug_detail(traceback.format_exc())
                raise
    
    def _setup_work_dirs(self, output_dir: Path, platform: str):
        """设置工作目录"""
        self.output_dir = output_dir / f"native-{platform}"
        self.work_dir = output_dir / f".native_build_{platform}"
        self.c_source_dir = self.work_dir / "c_source"
        self.obj_dir = self.work_dir / "obj"
        
        for d in [self.output_dir, self.work_dir, self.c_source_dir, self.obj_dir]:
            d.mkdir(parents=True, exist_ok=True)
            self.logger.trace_io(f"  {_('创建目录')}: {d}")
    
    def _collect_python_files(self, src_dir: Path) -> List[Path]:
        """收集 Python 源文件"""
        py_files = []
        for py_file in src_dir.rglob("*.py"):
            if py_file.is_file() and '__pycache__' not in str(py_file):
                py_files.append(py_file)
                self.logger.trace_io(f"  + {py_file.relative_to(src_dir)}")
        return py_files
    
    def _python_to_c(
        self,
        py_files: List[Path],
        src_dir: Path,
        main_script: str
    ) -> List[Tuple[Path, bool]]:
        """
        Python → C/C++ 转换
        
        使用 Cython 将 Python 代码转换为 C/C++ 代码
        
        Returns:
            List[Tuple[Path, bool]]: [(c_file_path, is_main), ...]
        """
        c_files = []
        main_file = (src_dir / main_script).resolve()
        
        # 检查 Cython 是否可用
        try:
            import Cython
            from Cython.Compiler import Main as CythonMain
            from Cython.Compiler.Options import CompilationOptions
            cython_available = True
            self.logger.debug_config(f"Cython {_('版本')}: {Cython.__version__}")
        except ImportError:
            cython_available = False
            self.logger.warn_minor("Cython {_('未安装')}, {_('使用内置转换器')}")
        
        for py_file in py_files:
            relative_path = py_file.relative_to(src_dir)
            c_file = self.c_source_dir / relative_path.with_suffix('.c')
            c_file.parent.mkdir(parents=True, exist_ok=True)
            
            is_main = py_file.resolve() == main_file
            
            if cython_available:
                # 使用 Cython 转换
                self._cython_compile(py_file, c_file)
            else:
                # 使用内置简易转换器
                self._builtin_convert(py_file, c_file, is_main)
            
            c_files.append((c_file, is_main))
            self.logger.trace_io(f"  {py_file.name} → {c_file.name}")
        
        # 生成主入口 C 文件（如果嵌入 Python）
        if self.config.embed_python:
            main_c = self._generate_main_wrapper(main_script, src_dir)
            c_files.append((main_c, True))
        
        return c_files
    
    def _cython_compile(self, py_file: Path, c_file: Path):
        """使用 Cython 编译"""
        cmd = [
            sys.executable, "-m", "cython",
            "-3",  # Python 3 语法
            "--embed" if self.config.embed_python else "",
            "-o", str(c_file),
            str(py_file)
        ]
        cmd = [c for c in cmd if c]  # 移除空字符串
        
        self.logger.trace_io(f"Cython: {' '.join(cmd)}")
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            self.logger.error_minimal(f"Cython {_('转换失败')}: {py_file}")
            self.logger.debug_detail(result.stderr)
            raise RuntimeError(f"Cython failed: {result.stderr}")
    
    def _builtin_convert(self, py_file: Path, c_file: Path, is_main: bool):
        """
        内置简易转换器 - 生成 C 包装代码
        
        这是一个简化的方案：将 Python 代码作为字符串嵌入到 C 程序中，
        运行时通过 Python C API 执行
        """
        py_code = py_file.read_text(encoding='utf-8')
        
        # 转义字符串
        escaped_code = py_code.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n"\n"')
        
        module_name = py_file.stem
        
        c_code = f'''
/* Auto-generated by Sikuwa Native Compiler */
/* Source: {py_file.name} */

#define PY_SSIZE_T_CLEAN
#include <Python.h>

static const char* {module_name}_source = 
"{escaped_code}";

PyObject* PyInit_{module_name}(void) {{
    PyObject* module = PyModule_Create(&{module_name}_def);
    if (module == NULL) return NULL;
    
    PyObject* code = Py_CompileString({module_name}_source, "{py_file.name}", Py_file_input);
    if (code == NULL) {{
        Py_DECREF(module);
        return NULL;
    }}
    
    PyObject* result = PyEval_EvalCode(code, PyModule_GetDict(module), PyModule_GetDict(module));
    Py_DECREF(code);
    
    if (result == NULL) {{
        Py_DECREF(module);
        return NULL;
    }}
    Py_DECREF(result);
    
    return module;
}}

static PyModuleDef {module_name}_def = {{
    PyModuleDef_HEAD_INIT,
    "{module_name}",
    NULL,
    -1,
    NULL
}};
'''
        
        c_file.write_text(c_code, encoding='utf-8')
    
    def _generate_main_wrapper(self, main_script: str, src_dir: Path) -> Path:
        """生成主入口 C 包装文件"""
        main_c = self.c_source_dir / "_sikuwa_main.c"
        
        # 读取主脚本
        main_py = src_dir / main_script
        main_code = main_py.read_text(encoding='utf-8')
        escaped_code = main_code.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n"\n"')
        
        c_code = f'''
/* Sikuwa Native Compiler - Main Entry Point */
/* Generated: {datetime.now().isoformat()} */

#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include <stdio.h>
#include <stdlib.h>

#ifdef _WIN32
#include <windows.h>
#define PATH_SEP "\\\\"
#else
#include <unistd.h>
#define PATH_SEP "/"
#endif

static const char* main_source = 
"{escaped_code}";

/* 获取可执行文件所在目录 */
static void get_exe_dir(char* buffer, size_t size) {{
#ifdef _WIN32
    GetModuleFileNameA(NULL, buffer, (DWORD)size);
    char* last_sep = strrchr(buffer, '\\\\');
    if (last_sep) *last_sep = '\\0';
#else
    ssize_t len = readlink("/proc/self/exe", buffer, size - 1);
    if (len != -1) {{
        buffer[len] = '\\0';
        char* last_sep = strrchr(buffer, '/');
        if (last_sep) *last_sep = '\\0';
    }} else {{
        buffer[0] = '.';
        buffer[1] = '\\0';
    }}
#endif
}}

int main(int argc, char* argv[]) {{
    char exe_dir[4096];
    get_exe_dir(exe_dir, sizeof(exe_dir));
    
    /* 设置 Python Home (如果存在 bundled Python) */
    char python_home[4096];
    snprintf(python_home, sizeof(python_home), "%s%spython", exe_dir, PATH_SEP);
    
#ifdef _WIN32
    wchar_t w_python_home[4096];
    MultiByteToWideChar(CP_UTF8, 0, python_home, -1, w_python_home, 4096);
    
    /* 检查是否存在 bundled Python */
    char python_dll[4096];
    snprintf(python_dll, sizeof(python_dll), "%s\\\\python{self.python_info.version.replace('.', '')}.dll", exe_dir);
    FILE* f = fopen(python_dll, "rb");
    if (f) {{
        fclose(f);
        Py_SetPythonHome(w_python_home);
    }}
#endif
    
    /* 初始化 Python */
    Py_Initialize();
    
    if (!Py_IsInitialized()) {{
        fprintf(stderr, "Error: Failed to initialize Python\\n");
        return 1;
    }}
    
    /* 设置 sys.argv */
    wchar_t** wargv = (wchar_t**)malloc(sizeof(wchar_t*) * argc);
    for (int i = 0; i < argc; i++) {{
        size_t len = strlen(argv[i]) + 1;
        wargv[i] = (wchar_t*)malloc(sizeof(wchar_t) * len);
        mbstowcs(wargv[i], argv[i], len);
    }}
    PySys_SetArgvEx(argc, wargv, 0);
    
    /* 添加当前目录到 sys.path */
    PyObject* sys_path = PySys_GetObject("path");
    PyObject* exe_dir_obj = PyUnicode_FromString(exe_dir);
    PyList_Insert(sys_path, 0, exe_dir_obj);
    Py_DECREF(exe_dir_obj);
    
    /* 执行主程序 */
    int result = PyRun_SimpleString(main_source);
    
    /* 清理 */
    for (int i = 0; i < argc; i++) {{
        free(wargv[i]);
    }}
    free(wargv);
    
    Py_Finalize();
    
    return result;
}}
'''
        
        main_c.write_text(c_code, encoding='utf-8')
        self.logger.debug_detail(f"{_('生成主入口文件')}: {main_c}")
        
        return main_c
    
    def _compile_c_files(self, c_files: List[Tuple[Path, bool]]) -> List[Path]:
        """编译 C/C++ 文件为目标文件"""
        obj_files = []
        
        for c_file, is_main in c_files:
            obj_file = self.obj_dir / c_file.with_suffix('.o').name
            
            # 选择编译器
            if c_file.suffix in ['.cpp', '.cxx', '.cc']:
                compiler = self.config.cxx
                flags = self.config.cxx_flags.copy()
            else:
                compiler = self.config.cc
                flags = self.config.c_flags.copy()
            
            # 添加 Python 头文件路径
            flags.append(f"-I{self.python_info.include_dir}")
            
            # 调试模式
            if self.config.debug:
                flags.extend(["-g", "-O0"])
            
            # 构建命令
            cmd = [compiler] + flags + ["-c", str(c_file), "-o", str(obj_file)]
            
            self.logger.trace_io(f"  {c_file.name} → {obj_file.name}")
            if self.verbose:
                self.logger.debug_detail(f"  $ {' '.join(cmd)}")
            
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                self.logger.error_minimal(f"{_('编译失败')}: {c_file.name}")
                self.logger.debug_detail(result.stderr)
                raise RuntimeError(f"Compilation failed: {c_file.name}\n{result.stderr}")
            
            obj_files.append(obj_file)
        
        return obj_files
    
    def _link_shared_library(
        self,
        obj_files: List[Path],
        project_name: str,
        platform: str
    ) -> Path:
        """链接生成动态链接库 (dll/so)"""
        
        # 确定输出文件名
        if platform == 'windows':
            dll_name = f"{project_name}.dll"
            import_lib = f"{project_name}.lib"
        elif platform == 'macos':
            dll_name = f"lib{project_name}.dylib"
            import_lib = None
        else:
            dll_name = f"lib{project_name}.so"
            import_lib = None
        
        dll_path = self.output_dir / dll_name
        
        # 选择链接器
        linker = self.config.cxx  # 使用 C++ 链接器
        
        # 构建链接命令
        link_flags = self.config.link_flags.copy()
        
        if platform == 'windows':
            link_flags.extend(["-shared", f"-Wl,--out-implib,{self.output_dir / import_lib}"])
        else:
            link_flags.append("-shared")
        
        # 添加 Python 库
        link_flags.append(f"-L{self.python_info.lib_dir}")
        link_flags.append(f"-l{self.python_info.lib_name}")
        
        # LTO 优化
        if self.config.lto:
            link_flags.append("-flto")
        
        # 剥离符号
        if self.config.strip and not self.config.debug:
            link_flags.append("-s")
        
        cmd = [linker] + [str(o) for o in obj_files] + link_flags + ["-o", str(dll_path)]
        
        self.logger.debug_detail(f"$ {' '.join(cmd)}")
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            self.logger.error_minimal(f"{_('链接失败')}: {dll_name}")
            self.logger.debug_detail(result.stderr)
            raise RuntimeError(f"Linking failed: {dll_name}\n{result.stderr}")
        
        return dll_path
    
    def _link_executable(
        self,
        obj_files: List[Path],
        project_name: str,
        platform: str
    ) -> Path:
        """链接生成可执行文件"""
        
        # 确定输出文件名
        if platform == 'windows':
            exe_name = f"{project_name}.exe"
        else:
            exe_name = project_name
        
        exe_path = self.output_dir / exe_name
        
        # 选择链接器
        linker = self.config.cxx
        
        # 构建链接命令
        link_flags = self.config.link_flags.copy()
        
        # 添加 Python 库
        link_flags.append(f"-L{self.python_info.lib_dir}")
        link_flags.append(f"-l{self.python_info.lib_name}")
        
        # Windows 特定
        if platform == 'windows':
            link_flags.extend(["-lws2_32", "-ladvapi32", "-lshell32"])
        
        # Linux 特定
        if platform == 'linux':
            link_flags.extend(["-lpthread", "-ldl", "-lutil", "-lm"])
        
        # LTO 优化
        if self.config.lto:
            link_flags.append("-flto")
        
        # 剥离符号
        if self.config.strip and not self.config.debug:
            link_flags.append("-s")
        
        cmd = [linker] + [str(o) for o in obj_files] + link_flags + ["-o", str(exe_path)]
        
        self.logger.debug_detail(f"$ {' '.join(cmd)}")
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            self.logger.error_minimal(f"{_('链接失败')}: {exe_name}")
            self.logger.debug_detail(result.stderr)
            raise RuntimeError(f"Linking failed: {exe_name}\n{result.stderr}")
        
        return exe_path
    
    def _copy_runtime_deps(self, platform: str):
        """复制运行时依赖"""
        
        # 复制 Python DLL (如果需要)
        if not self.config.python_static and self.config.embed_python:
            if platform == 'windows':
                python_dll = Path(sys.prefix) / self.python_info.dll_name
                if python_dll.exists():
                    dest = self.output_dir / python_dll.name
                    shutil.copy2(python_dll, dest)
                    self.logger.trace_io(f"  {_('复制')}: {python_dll.name}")
                
                # 复制 vcruntime
                vcruntime = Path(sys.prefix) / "vcruntime140.dll"
                if vcruntime.exists():
                    shutil.copy2(vcruntime, self.output_dir / vcruntime.name)
                    self.logger.trace_io(f"  {_('复制')}: vcruntime140.dll")
            
            elif platform == 'linux':
                python_so = self.python_info.lib_dir / self.python_info.dll_name
                if python_so.exists():
                    dest = self.output_dir / python_so.name
                    shutil.copy2(python_so, dest)
                    self.logger.trace_io(f"  {_('复制')}: {python_so.name}")
        
        # 复制标准库 (如果嵌入 Python)
        if self.config.embed_python:
            self._copy_stdlib(platform)
    
    def _copy_stdlib(self, platform: str):
        """复制 Python 标准库"""
        stdlib_dest = self.output_dir / "python_lib"
        stdlib_dest.mkdir(exist_ok=True)
        
        # 复制 zip 格式的标准库 (如果存在)
        if platform == 'windows':
            stdlib_zip = Path(sys.prefix) / f"python{self.python_info.version.replace('.', '')}.zip"
            if stdlib_zip.exists():
                shutil.copy2(stdlib_zip, self.output_dir / stdlib_zip.name)
                self.logger.trace_io(f"  {_('复制')}: {stdlib_zip.name}")
                return
        
        # 复制关键标准库模块
        essential_modules = [
            'os.py', 'sys.py', 'io.py', 'abc.py', 'functools.py',
            'collections', 'encodings', 'importlib'
        ]
        
        stdlib_src = Path(self.python_info.stdlib_dir)
        for module in essential_modules:
            src = stdlib_src / module
            if src.exists():
                if src.is_dir():
                    shutil.copytree(src, stdlib_dest / module, dirs_exist_ok=True)
                else:
                    shutil.copy2(src, stdlib_dest / module)
                self.logger.trace_io(f"  {_('复制')}: {module}")
    
    def _cleanup(self):
        """清理临时文件"""
        if self.work_dir and self.work_dir.exists():
            shutil.rmtree(self.work_dir)
            self.logger.trace_io(f"  {_('清理')}: {self.work_dir}")


def detect_compiler() -> Tuple[str, str]:
    """检测系统中可用的 C/C++ 编译器"""
    
    # Windows 优先检测 MinGW/MSYS2
    if sys.platform == 'win32':
        compilers = [
            ('gcc', 'g++'),
            ('clang', 'clang++'),
            ('cl', 'cl'),  # MSVC
        ]
    else:
        compilers = [
            ('gcc', 'g++'),
            ('clang', 'clang++'),
        ]
    
    for cc, cxx in compilers:
        try:
            result = subprocess.run([cc, '--version'], capture_output=True, timeout=5)
            if result.returncode == 0:
                return cc, cxx
        except (FileNotFoundError, subprocess.TimeoutExpired):
            continue
    
    raise RuntimeError("No C/C++ compiler found. Please install GCC, Clang, or MSVC.")


def native_build(
    project_name: str,
    src_dir: str,
    main_script: str,
    output_dir: str,
    platform: str,
    compiler_config: Optional[CompilerConfig] = None,
    verbose: bool = False
) -> Dict[str, Path]:
    """
    执行原生编译
    
    Args:
        project_name: 项目名称
        src_dir: 源代码目录
        main_script: 主脚本路径
        output_dir: 输出目录
        platform: 目标平台
        compiler_config: 编译器配置
        verbose: 详细输出
    
    Returns:
        Dict[str, Path]: 生成的文件路径
    """
    if compiler_config is None:
        cc, cxx = detect_compiler()
        compiler_config = CompilerConfig(cc=cc, cxx=cxx)
    
    compiler = NativeCompiler(compiler_config, verbose=verbose)
    
    return compiler.compile_project(
        project_name=project_name,
        src_dir=Path(src_dir),
        main_script=main_script,
        output_dir=Path(output_dir),
        platform=platform
    )
