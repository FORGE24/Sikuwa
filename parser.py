# parser.py
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional


@dataclass
class NuitkaConfig:
    """Nuitka 编译参数（对应 build.ski 中的 nuitka 块）"""
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
    """项目构建配置（解析 build.ski 后）"""
    project_name: str = "MyProject"
    version: str = "1.0.0"
    src_dir: str = "src"
    main_script: str = "main.py"
    output_dir: str = "dist"
    build_dir: str = "build"
    resources: List[Dict[str, str]] = field(default_factory=list)
    platforms: List[str] = field(default_factory=lambda: ["current"])
    pre_build_commands: List[str] = field(default_factory=list)
    post_build_commands: List[str] = field(default_factory=list)
    nuitka: NuitkaConfig = field(default_factory=NuitkaConfig)


class SkiParser:
    """解析 build.ski 配置文件"""

    def __init__(self, config_path: str = "build.ski"):
        self.config_path = Path(config_path)
        self.config = BuildConfig()
        self._parse()

    def _load_content(self) -> str:
        if not self.config_path.exists():
            raise FileNotFoundError(f"未找到配置文件: {self.config_path}\n请创建 build.ski")

        content = self.config_path.read_text(encoding="utf-8")

        # 移除注释（# 开头）
        content = re.sub(r"#.*", "", content)

        return content

    def _parse(self) -> None:
        content = self._load_content()

        self._parse_top_level(content)
        self._parse_nuitka_block(content)
        self._parse_resources(content)
        self._parse_commands(content)
        self._parse_platforms(content)

    def _parse_top_level(self, content: str) -> None:
        """解析顶层配置"""
        patterns = {
            "project_name": r"project\s*=\s*['\"]([^'\"]+)['\"]",
            "version": r"version\s*=\s*['\"]([^'\"]+)['\"]",
            "src_dir": r"srcDir\s*=\s*['\"]([^'\"]+)['\"]",
            "main_script": r"mainScript\s*=\s*['\"]([^'\"]+)['\"]",
            "output_dir": r"outputDir\s*=\s*['\"]([^'\"]+)['\"]",
            "build_dir": r"buildDir\s*=\s*['\"]([^'\"]+)['\"]",
        }
        for key, pattern in patterns.items():
            match = re.search(pattern, content, flags=re.IGNORECASE)
            if match:
                setattr(self.config, key, match.group(1))

    def _parse_nuitka_block(self, content: str) -> None:
        """解析 nuitka {...} 区块"""
        block = re.search(r"nuitka\s*\{([^}]*)\}", content, flags=re.DOTALL | re.IGNORECASE)
        if not block:
            return

        body = block.group(1)
        cfg = self.config.nuitka

        # Bool
        bools = {
            "standalone": r"standalone\s*=\s*(true|false)",
            "follow_imports": r"followImports\s*=\s*(true|false)",
            "remove_output": r"removeOutput\s*=\s*(true|false)",
            "show_progress": r"showProgress\s*=\s*(true|false)",
        }
        for key, pattern in bools.items():
            m = re.search(pattern, body, flags=re.IGNORECASE)
            if m:
                setattr(cfg, key, m.group(1).lower() == "true")

        # List
        lists = {
            "include_packages": r"includePackages\s*=\s*\[(.*?)\]",
            "include_modules": r"includeModules\s*=\s*\[(.*?)\]",
            "exclude_modules": r"excludeModules\s*=\s*\[(.*?)\]",
        }
        for key, pattern in lists.items():
            m = re.search(pattern, body, flags=re.DOTALL | re.IGNORECASE)
            if m:
                items = [
                    item.strip().strip("'\"")
                    for item in m.group(1).split(",")
                    if item.strip()
                ]
                setattr(cfg, key, items)

        # String
        strings = {
            "windows_icon": r"windowsIcon\s*=\s*['\"]([^'\"]+)['\"]",
            "windows_company": r"windowsCompany\s*=\s*['\"]([^'\"]+)['\"]",
            "windows_product": r"windowsProduct\s*=\s*['\"]([^'\"]+)['\"]",
            "macos_app_name": r"macosAppName\s*=\s*['\"]([^'\"]+)['\"]",
        }
        for key, pattern in strings.items():
            m = re.search(pattern, body, flags=re.IGNORECASE)
            if m:
                setattr(cfg, key, m.group(1))

    def _parse_resources(self, content: str) -> None:
        """解析资源映射"""
        block = re.search(r"resources\s*\{([^}]*)\}", content, flags=re.DOTALL | re.IGNORECASE)
        if not block:
            return

        pattern = r"from\s*['\"]([^'\"]+)['\"]\s+to\s*['\"]([^'\"]+)['\"]"
        matches = re.findall(pattern, block.group(1))
        self.config.resources = [{"src": src, "dest": dest} for src, dest in matches]

    def _parse_commands(self, content: str) -> None:
        """解析 preBuild / postBuild"""

        def parse_command(block_name: str) -> List[str]:
            pattern = rf"{block_name}\s*\{{[^}}]*commands\s*=\s*\[(.*?)\]"
            block = re.search(pattern, content, flags=re.DOTALL | re.IGNORECASE)
            if not block:
                return []
            entries = [
                cmd.strip().strip("'\"")
                for cmd in block.group(1).split(",")
                if cmd.strip()
            ]
            return entries

        self.config.pre_build_commands = parse_command("preBuild")
        self.config.post_build_commands = parse_command("postBuild")

    def _parse_platforms(self, content: str) -> None:
        """解析构建平台列表"""
        m = re.search(r"platforms\s*=\s*\[(.*?)\]", content, flags=re.DOTALL | re.IGNORECASE)
        if m:
            self.config.platforms = [
                p.strip().strip("'\"")
                for p in m.group(1).split(",")
                if p.strip()
            ]
