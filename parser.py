# sikuwa/parser.py
"""
Sikuwa 配置解析器
"""
from pathlib import Path
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, field


@dataclass
class NuitkaConfig:
    """Nuitka 专属配置"""
    standalone: bool = True
    follow_imports: bool = True
    remove_output: bool = True
    show_progress: bool = True
    include_packages: List[str] = field(default_factory=list)
    include_modules: List[str] = field(default_factory=list)
    exclude_modules: List[str] = field(default_factory=list)
    windows_icon: Optional[str] = None
    windows_company: Optional[str] = None
    windows_product: Optional[str] = None
    macos_app_name: Optional[str] = None


@dataclass
class BuildConfig:
    """构建配置（完整版）"""
    # 基础配置
    project_name: str
    main_script: str
    version: str = "1.0.0"
    
    # 目录配置
    src_dir: str = "."
    output_dir: str = "dist"
    build_dir: str = "build"
    
    # 平台配置
    platforms: List[str] = field(default_factory=lambda: ["windows", "linux"])
    
    # Nuitka 配置
    nuitka: NuitkaConfig = field(default_factory=NuitkaConfig)
    
    # 资源配置
    resources: List[Dict[str, str]] = field(default_factory=list)
    
    # 构建钩子
    pre_build_commands: List[str] = field(default_factory=list)
    post_build_commands: List[str] = field(default_factory=list)
    
    # 向后兼容属性
    @property
    def name(self) -> str:
        return self.project_name
    
    @property
    def entry(self) -> str:
        return self.main_script
    
    # 兼容旧版属性
    icon: Optional[str] = None
    console: bool = False
    onefile: bool = True
    strip: bool = True
    upx: bool = False
    follow_imports: bool = True
    standalone: bool = True
    include_data: List[tuple] = field(default_factory=list)
    include_modules: List[str] = field(default_factory=list)
    exclude_modules: List[str] = field(default_factory=list)
    product_name: Optional[str] = None
    product_version: str = "1.0.0"
    company_name: Optional[str] = None
    file_description: Optional[str] = None
    extra_args: List[str] = field(default_factory=list)


class ConfigParser:
    """配置文件解析器"""
    
    def __init__(self, config_path: Path):
        self.config_path = config_path
        self.config: Dict[str, Any] = {}
    
    def parse(self) -> BuildConfig:
        """解析配置文件"""
        if not self.config_path.exists():
            raise FileNotFoundError(f"配置文件不存在: {self.config_path}")
        
        file_content = self.config_path.read_text(encoding='utf-8')
        
        if self.config_path.suffix == '.toml':
            self.config = self._parse_toml(file_content)
        elif self.config_path.suffix in ['.yaml', '.yml']:
            self.config = self._parse_yaml(file_content)
        elif self.config_path.suffix == '.json':
            self.config = self._parse_json(file_content)
        else:
            raise ValueError(f"不支持的配置文件格式: {self.config_path.suffix}")
        
        return self._to_build_config()
    
    def _parse_toml(self, file_content: str) -> Dict:
        """解析 TOML 配置"""
        try:
            import tomllib
        except ImportError:
            try:
                import tomli as tomllib
            except ImportError:
                raise ImportError("需要安装 tomli: pip install tomli")
        
        return tomllib.loads(file_content)
    
    def _parse_yaml(self, file_content: str) -> Dict:
        """解析 YAML 配置"""
        try:
            import yaml
            return yaml.safe_load(file_content)
        except ImportError:
            raise ImportError("需要安装 PyYAML: pip install pyyaml")
    
    def _parse_json(self, file_content: str) -> Dict:
        """解析 JSON 配置"""
        import json
        return json.loads(file_content)
    
    def _to_build_config(self) -> BuildConfig:
        """转换为 BuildConfig 对象"""
        build_section = self.config.get('build', {})
        
        config_name = build_section.get('name')
        config_entry = build_section.get('entry')
        
        if not config_name:
            raise ValueError("配置文件缺少必需字段: build.name")
        if not config_entry:
            raise ValueError("配置文件缺少必需字段: build.entry")
        
        # 创建 Nuitka 配置
        nuitka_section = build_section.get('nuitka', {})
        nuitka_config = NuitkaConfig(
            standalone=nuitka_section.get('standalone', True),
            follow_imports=nuitka_section.get('follow_imports', True),
            remove_output=nuitka_section.get('remove_output', True),
            show_progress=nuitka_section.get('show_progress', True),
            include_packages=nuitka_section.get('include_packages', []),
            include_modules=nuitka_section.get('include_modules', []),
            exclude_modules=nuitka_section.get('exclude_modules', []),
            windows_icon=build_section.get('icon'),
            windows_company=build_section.get('company_name'),
            windows_product=build_section.get('product_name', config_name),
            macos_app_name=nuitka_section.get('macos_app_name')
        )
        
        # 处理资源文件
        resources = []
        data_files = nuitka_section.get('include_data', [])
        if data_files:
            for item in data_files:
                if isinstance(item, dict):
                    resources.append({
                        'src': item.get('src', ''),
                        'dest': item.get('dst', item.get('dest', ''))
                    })
        
        # 获取产品信息
        product_section = self.config.get('product', {})
        
        # 创建完整配置
        result = BuildConfig(
            project_name=config_name,
            main_script=config_entry,
            version=product_section.get('version', '1.0.0'),
            src_dir=build_section.get('src_dir', '.'),
            output_dir=build_section.get('output_dir', 'dist'),
            build_dir=build_section.get('build_dir', 'build'),
            platforms=build_section.get('platforms', ['windows', 'linux']),
            nuitka=nuitka_config,
            resources=resources,
            pre_build_commands=build_section.get('pre_build_commands', []),
            post_build_commands=build_section.get('post_build_commands', []),
            
            # 兼容旧版属性
            icon=build_section.get('icon'),
            console=build_section.get('console', False),
            onefile=build_section.get('onefile', True),
            strip=build_section.get('strip', True),
            upx=build_section.get('upx', False),
            follow_imports=nuitka_section.get('follow_imports', True),
            standalone=nuitka_section.get('standalone', True),
            include_modules=nuitka_section.get('include_modules', []),
            exclude_modules=nuitka_section.get('exclude_modules', []),
            product_name=product_section.get('name', config_name),
            product_version=product_section.get('version', '1.0.0'),
            company_name=product_section.get('company'),
            file_description=product_section.get('description'),
            extra_args=build_section.get('extra_args', [])
        )
        
        # 处理 include_data 兼容性
        if data_files:
            result.include_data = [
                (item['src'], item.get('dst', item.get('dest', ''))) 
                for item in data_files if isinstance(item, dict)
            ]
        
        return result


def parse_config(config_path: Path) -> BuildConfig:
    """快速解析配置文件"""
    parser = ConfigParser(config_path)
    return parser.parse()


def create_default_config(output_path: Path, format_type: str = 'toml'):
    """创建默认配置文件"""
    if format_type == 'toml':
        template_content = '''[build]
name = "myapp"
entry = "main.py"
src_dir = "."
output_dir = "dist"
build_dir = "build"
platforms = ["windows", "linux"]
console = false
onefile = true
[build.nuitka]
standalone = true
follow_imports = true
remove_output = true
show_progress = true
include_packages = []
include_modules = []
exclude_modules = []
[product]
name = "My Application"
version = "1.0.0"
company = "Your Company"
description = "Application description"
'''
    elif format_type == 'yaml':
        template_content = '''build:
  name: myapp
  entry: main.py
  src_dir: "."
  output_dir: dist
  build_dir: build
  platforms: [windows, linux]
  console: false
  onefile: true
  
  nuitka:
    standalone: true
    follow_imports: true
    remove_output: true
    show_progress: true
product:
  name: My Application
  version: 1.0.0
  company: Your Company
  description: Application description
'''
    elif format_type == 'json':
        template_content = '''{
  "build": {
    "name": "myapp",
    "entry": "main.py",
    "src_dir": ".",
    "output_dir": "dist",
    "build_dir": "build",
    "platforms": ["windows", "linux"],
    "console": false,
    "onefile": true,
    "nuitka": {
      "standalone": true,
      "follow_imports": true,
      "remove_output": true,
      "show_progress": true
    }
  },
  "product": {
    "name": "My Application",
    "version": "1.0.0",
    "company": "Your Company",
    "description": "Application description"
  }
}'''
    else:
        raise ValueError(f"不支持的格式: {format_type}")
    
    output_path.write_text(template_content.strip(), encoding='utf-8')


def validate_config(config: BuildConfig) -> List[str]:
    """验证配置有效性"""
    errors = []
    
    # 检查源码目录
    src_dir = Path(config.src_dir)
    if not src_dir.exists():
        errors.append(f"源码目录不存在: {config.src_dir}")
    
    # 检查入口文件
    entry_file = src_dir / config.main_script
    if not entry_file.exists():
        errors.append(f"入口文件不存在: {entry_file}")
    
    # 检查图标文件
    if config.icon:
        icon_file = Path(config.icon)
        if not icon_file.exists():
            errors.append(f"图标文件不存在: {config.icon}")
    
    return errors
