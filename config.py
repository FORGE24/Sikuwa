# sikuwa/config.py
"""
Sikuwa 配置管理模块
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import List, Optional, Dict, Any, TYPE_CHECKING
from dataclasses import dataclass, field, asdict

# 修复 tomli 导入，避免 mypyc 问题
if sys.version_info >= (3, 11):
    import tomllib
else:
    try:
        import tomli as tomllib
    except ImportError:
        raise ImportError(
            "Python < 3.11 需要安装 tomli:\n"
            "  pip install tomli\n"
            "或升级到 Python 3.11+"
        )


@dataclass
class NuitkaOptions:
    """Nuitka 编译选项"""
    
    # 基础选项
    standalone: bool = True
    onefile: bool = False
    follow_imports: bool = True
    show_progress: bool = True
    enable_console: bool = True
    
    # 优化选项
    optimize: bool = True
    lto: bool = False  # Link Time Optimization
    
    # 平台特定选项
    windows_icon: Optional[str] = None
    windows_company_name: Optional[str] = None
    windows_product_name: Optional[str] = None
    windows_file_version: Optional[str] = None
    windows_product_version: Optional[str] = None
    
    macos_app_bundle: bool = False
    macos_icon: Optional[str] = None
    
    # 包含/排除选项
    include_packages: List[str] = field(default_factory=list)
    include_modules: List[str] = field(default_factory=list)
    include_data_files: List[str] = field(default_factory=list)
    include_data_dirs: List[Dict[str, str]] = field(default_factory=list)
    
    nofollow_imports: List[str] = field(default_factory=list)
    nofollow_import_to: List[str] = field(default_factory=list)
    
    # 插件选项
    enable_plugins: List[str] = field(default_factory=list)
    disable_plugins: List[str] = field(default_factory=list)
    
    # 额外参数
    extra_args: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        data = asdict(self)
        
        # 过滤掉值为 None 的字段，避免 TOML 序列化错误
        filtered_data = {}
        for key, value in data.items():
            if value is not None:
                filtered_data[key] = value
        
        return filtered_data
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'NuitkaOptions':
        """从字典创建"""
        # 过滤掉不存在的字段
        valid_fields = {f.name for f in cls.__dataclass_fields__.values()}
        filtered_data = {k: v for k, v in data.items() if k in valid_fields}
        return cls(**filtered_data)


@dataclass
class NativeCompilerOptions:
    """原生编译器选项 - Python → C/C++ → GCC/G++ → dll/so + exe"""
    
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
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'NativeCompilerOptions':
        """从字典创建"""
        valid_fields = {f.name for f in cls.__dataclass_fields__.values()}
        filtered_data = {k: v for k, v in data.items() if k in valid_fields}
        return cls(**filtered_data)


@dataclass
class BuildConfig:
    """Sikuwa 构建配置"""
    
    # 项目基础信息
    project_name: str
    version: str = "1.0.0"
    description: str = ""
    author: str = ""
    
    # 构建配置
    main_script: str = "main.py"
    src_dir: str = "."
    output_dir: str = "dist"
    build_dir: str = "build"
    
    # 目标平台
    platforms: List[str] = field(default_factory=lambda: ["windows"])
    
    # 编译模式选择: "nuitka" | "native"
    compiler_mode: str = "nuitka"
    
    # Nuitka 选项 (compiler_mode="nuitka" 时使用)
    nuitka_options: NuitkaOptions = field(default_factory=NuitkaOptions)
    
    # 原生编译器选项 (compiler_mode="native" 时使用)
    native_options: NativeCompilerOptions = field(default_factory=NativeCompilerOptions)
    
    # 资源文件
    resources: List[str] = field(default_factory=list)
    
    # Python 环境
    python_version: Optional[str] = None
    python_path: Optional[str] = None
    
    # 依赖管理
    requirements_file: Optional[str] = None
    pip_index_url: Optional[str] = None
    dependencies: List[str] = field(default_factory=list)

    # 编译序列配置
    build_sequence: Optional[List[Dict[str, Any]]] = None
    sequence_dependencies: Optional[Dict[str, List[str]]] = None
    parallel_build: bool = False
    max_workers: int = 4
    
    # 钩子脚本
    pre_build_script: Optional[str] = None
    post_build_script: Optional[str] = None
    
    def validate(self) -> None:
        """验证配置"""
        if not self.project_name:
            raise ValueError("project_name 不能为空")
        
        # 如果是编译序列配置，跳过main_script验证
        if not self.build_sequence:
            if not self.main_script:
                raise ValueError("main_script 不能为空")
            
            valid_platforms = ["windows", "linux", "macos"]
            for platform in self.platforms:
                if platform not in valid_platforms:
                    raise ValueError(f"不支持的平台: {platform}，有效平台: {valid_platforms}")
            
            # 检查主脚本是否存在
            main_file = Path(self.src_dir) / self.main_script
            if not main_file.exists():
                raise FileNotFoundError(f"主脚本不存在: {main_file}")
    
    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        data = asdict(self)
        data['nuitka_options'] = self.nuitka_options.to_dict()
        data['native_options'] = self.native_options.to_dict()
        
        # 过滤掉值为 None 的字段，避免 TOML 序列化错误
        filtered_data = {}
        for key, value in data.items():
            if value is not None:
                filtered_data[key] = value
        
        return filtered_data
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'BuildConfig':
        """从字典创建"""
        # 提取 nuitka_options
        nuitka_data = data.pop('nuitka_options', {})
        nuitka_options = NuitkaOptions.from_dict(nuitka_data)
        
        # 提取 native_options
        native_data = data.pop('native_options', {})
        native_options = NativeCompilerOptions.from_dict(native_data)
        
        # 过滤掉不存在的字段
        valid_fields = {f.name for f in cls.__dataclass_fields__.values()}
        filtered_data = {k: v for k, v in data.items() if k in valid_fields}
        
        return cls(nuitka_options=nuitka_options, native_options=native_options, **filtered_data)
    
    @classmethod
    def from_toml(cls, config_file: str) -> 'BuildConfig':
        """从 TOML 文件加载配置"""
        config_path = Path(config_file)
        
        if not config_path.exists():
            raise FileNotFoundError(f"配置文件不存在: {config_file}")
        
        try:
            with open(config_path, 'rb') as f:
                data = tomllib.load(f)
        except Exception as e:
            raise ValueError(f"解析 TOML 文件失败: {e}")
        
        # 提取 [sikuwa] 部分
        if 'sikuwa' not in data:
            raise ValueError("配置文件缺少 [sikuwa] 部分")
        
        sikuwa_config = data['sikuwa'].copy()
        
        # 解析 nuitka 选项
        nuitka_data = sikuwa_config.pop('nuitka', {})
        
        # 处理可能存在的嵌套 nuitka_options
        if 'nuitka_options' in sikuwa_config:
            nuitka_data.update(sikuwa_config.pop('nuitka_options'))
        
        nuitka_options = NuitkaOptions.from_dict(nuitka_data)
        
        # 解析原生编译器选项
        native_data = sikuwa_config.pop('native', {})
        
        # 处理可能存在的嵌套 native_options
        if 'native_options' in sikuwa_config:
            native_data.update(sikuwa_config.pop('native_options'))
        
        native_options = NativeCompilerOptions.from_dict(native_data)
        
        # 创建配置对象
        config = cls(nuitka_options=nuitka_options, native_options=native_options, **sikuwa_config)
        
        return config
    
    def save_to_toml(self, config_file: str) -> None:
        """保存配置到 TOML 文件"""
        # 优先使用 tomli_w
        try:
            import tomli_w as toml_writer
            use_binary = True
        except ImportError:
            try:
                import toml as toml_writer
                use_binary = False
            except ImportError:
                raise ImportError(
                    "需要安装 'tomli-w' 或 'toml' 包以保存 TOML 文件:\n"
                    "  pip install tomli-w\n"
                    "或\n"
                    "  pip install toml"
                )
        
        data = {
            'sikuwa': self.to_dict()
        }
        
        config_path = Path(config_file)
        
        try:
            if use_binary:
                with open(config_path, 'wb') as f:
                    toml_writer.dump(data, f)
            else:
                with open(config_path, 'w', encoding='utf-8') as f:
                    toml_writer.dump(data, f)
        except Exception as e:
            raise IOError(f"保存配置文件失败: {e}")


class ConfigManager:
    """配置管理器"""
    
    DEFAULT_CONFIG_FILES = [
        "sikuwa.toml",
        "pyproject.toml",
        ".sikuwa.toml"
    ]
    
    @staticmethod
    def find_config() -> Optional[Path]:
        """自动查找配置文件"""
        for config_file in ConfigManager.DEFAULT_CONFIG_FILES:
            config_path = Path(config_file)
            if config_path.exists():
                return config_path
        return None
    
    @staticmethod
    def load_config(config_file: Optional[str] = None) -> BuildConfig:
        """加载配置文件"""
        if config_file:
            return BuildConfig.from_toml(config_file)
        
        # 自动查找配置文件
        config_path = ConfigManager.find_config()
        if config_path:
            return BuildConfig.from_toml(str(config_path))
        
        raise FileNotFoundError(
            "未找到配置文件，请创建以下文件之一:\n  " +
            "\n  ".join(ConfigManager.DEFAULT_CONFIG_FILES) +
            "\n\n使用命令创建默认配置:\n  sikuwa init"
        )
    
    @staticmethod
    def create_default_config(output_file: str = "sikuwa.toml") -> None:
        """创建默认配置文件"""
        default_config = BuildConfig(
            project_name="my_project",
            version="1.0.0",
            description="My Python Project",
            author="",
            main_script="main.py",
            src_dir=".",
            output_dir="dist",
            build_dir="build",
            platforms=["windows"],
            nuitka_options=NuitkaOptions(
                standalone=True,
                onefile=False,
                follow_imports=True,
                show_progress=True,
                enable_console=True,
                optimize=True,
                include_packages=[],
                nofollow_import_to=[
                    "numpy",
                    "pandas",
                    "matplotlib"
                ]
            ),
            resources=[],
            dependencies=["requests>=2.0.0", "click>=8.0.0"]
        )
        
        try:
            default_config.save_to_toml(output_file)
            print(f"✓ 已创建默认配置文件: {output_file}")
        except Exception as e:
            print(f"✗ 创建配置文件失败: {e}")
            raise


# 便捷函数
def load_config(config_file: Optional[str] = None) -> BuildConfig:
    """加载配置（便捷函数）"""
    return ConfigManager.load_config(config_file)


def create_config(output_file: str = "sikuwa.toml") -> None:
    """创建默认配置（便捷函数）"""
    ConfigManager.create_default_config(output_file)


def validate_config(config: BuildConfig) -> List[str]:
    """验证配置有效性，返回错误列表"""
    errors = []
    
    try:
        config.validate()
    except Exception as e:
        errors.append(str(e))
    
    # 额外检查
    src_dir = Path(config.src_dir)
    if not src_dir.exists():
        errors.append(f"源码目录不存在: {config.src_dir}")
    
    if config.nuitka_options.windows_icon:
        icon_path = Path(config.nuitka_options.windows_icon)
        if not icon_path.exists():
            errors.append(f"图标文件不存在: {config.nuitka_options.windows_icon}")
    
    return errors


"""Sikuwa 配置管理"""

from pathlib import Path
from typing import List, Dict, Optional


class SikuwaConfig:
    """Sikuwa 项目配置"""
    
    def __init__(self, config_path: Path = None):
        if config_path is None:
            config_path = Path("sikuwa.toml")
        
        self.config_path = config_path
        self._load_config()
    
    def _load_config(self):
        """加载配置文件"""
        if not self.config_path.exists():
            raise FileNotFoundError(f"配置文件不存在: {self.config_path}")
        
        with open(self.config_path, 'rb') as f:
            data = tomllib.load(f)
        
        # 基础配置
        sikuwa = data.get('sikuwa', {})
        self.project_name = sikuwa.get('project_name', 'my_project')
        self.version = sikuwa.get('version', '1.0.0')
        self.main_script = Path(sikuwa.get('main_script', 'main.py'))
        self.src_dir = Path(sikuwa.get('src_dir', '.'))
        self.output_dir = Path(sikuwa.get('output_dir', 'dist'))
        self.build_dir = Path(sikuwa.get('build_dir', 'build'))
        self.platforms = sikuwa.get('platforms', ['windows'])
        
        # Nuitka 配置
        nuitka = data.get('sikuwa', {}).get('nuitka', {})
        self.standalone = nuitka.get('standalone', True)
        self.onefile = nuitka.get('onefile', False)
        self.follow_imports = nuitka.get('follow_imports', True)
        self.show_progress = nuitka.get('show_progress', True)
        self.enable_console = nuitka.get('enable_console', True)
        
        self.include_packages = nuitka.get('include_packages', [])
        self.include_data_files = nuitka.get('include_data_files', [])
        self.include_data_dirs = nuitka.get('include_data_dirs', [])  # 新增
        
        self.extra_args = nuitka.get('extra_args', [])
    
    def __repr__(self):
        return f"<SikuwaConfig project={self.project_name} version={self.version}>"

if __name__ == '__main__':
    # 测试配置模块
    print("Sikuwa Config - 测试模式")
    print("=" * 70)
    
    # 创建测试配置
    test_config = BuildConfig(
        project_name="test_app",
        version="0.1.0",
        main_script="main.py",
        platforms=["windows", "linux"],
        nuitka_options=NuitkaOptions(
            standalone=True,
            onefile=True,
            follow_imports=True,
            include_packages=["requests", "click"],
            windows_icon="icon.ico",
            nofollow_import_to=["numpy", "pandas"]
        ),
        resources=["config.json", "data/"]
    )
    
    print("\n测试配置对象:")
    print(f"  项目名称: {test_config.project_name}")
    print(f"  版本: {test_config.version}")
    print(f"  目标平台: {test_config.platforms}")
    print(f"  Standalone: {test_config.nuitka_options.standalone}")
    print(f"  OneFile: {test_config.nuitka_options.onefile}")
    
    # 测试保存和加载
    test_file = "test_sikuwa.toml"
    try:
        print(f"\n保存配置到: {test_file}")
        test_config.save_to_toml(test_file)
        
        print(f"从文件加载配置: {test_file}")
        loaded_config = BuildConfig.from_toml(test_file)
        
        print("\n加载的配置:")
        print(f"  项目名称: {loaded_config.project_name}")
        print(f"  版本: {loaded_config.version}")
        print(f"  目标平台: {loaded_config.platforms}")
        print(f"  包含包: {loaded_config.nuitka_options.include_packages}")
        print(f"  排除包: {loaded_config.nuitka_options.nofollow_import_to}")
        
        print("\n✓ 配置模块测试通过!")
        
    except Exception as e:
        print(f"\n✗ 测试失败: {e}")
        import traceback
        traceback.print_exc()
        
    finally:
        # 清理测试文件
        import os
        if os.path.exists(test_file):
            os.remove(test_file)
            print(f"\n清理测试文件: {test_file}")
