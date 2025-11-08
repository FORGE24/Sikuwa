# builder.py
import os
import sys
import shutil
import subprocess
import logging
import hashlib
from pathlib import Path
from typing import List, Dict, Optional
from .parser import BuildConfig
from datetime import datetime

LOGS_DIR_NAME = "sikuwa_logs"

class SikuwaBuilder:
    """执行构建逻辑（增强版：更鲁棒的日志与错误处理）"""
    def __init__(self, config: BuildConfig, verbose: bool = False):
        self.config = config
        self.project_root = Path.cwd()
        self.src_dir = self.project_root / config.src_dir
        self.main_script = self.src_dir / config.main_script
        self.output_root = self.project_root / config.output_dir
        self.build_root = self.project_root / config.build_dir
        self.verbose = verbose

        # 日志目录
        self.logs_dir = self.project_root / LOGS_DIR_NAME
        self.logs_dir.mkdir(parents=True, exist_ok=True)

        # 初始化日志（显示构建进度）
        self._init_logger()
        self._check_main_script()

    def _init_logger(self) -> None:
        log_file = self.logs_dir / f"build-{datetime.utcnow().strftime('%Y%m%dT%H%M%SZ')}.log"
        level = logging.DEBUG if self.verbose else logging.INFO
        logging.basicConfig(
            level=level,
            format=f"[{self.config.project_name}] %(levelname)s: %(message)s",
            handlers=[
                logging.StreamHandler(sys.stdout),
                logging.FileHandler(str(log_file), encoding="utf-8")
            ]
        )
        self.logger = logging.getLogger("Sikuwa")
        self.logger.debug(f"日志文件: {log_file}")

    def _check_main_script(self) -> None:
        """检查入口脚本是否存在"""
        if not self.main_script.exists():
            raise FileNotFoundError(
                f"入口脚本不存在: {self.main_script}\n"
                f"请检查 build.ski 中的 mainScript 配置或创建该文件"
            )

    def _get_platform_dir(self, platform: str) -> Path:
        """获取平台专属输出目录"""
        plat = platform
        if platform == "current":
            plat = sys.platform.replace("win32", "windows").replace("darwin", "macos")
        return self.output_root / f"{self.config.project_name}-v{self.config.version}-{plat}"

    def _run_commands(self, commands: List[str], env: Dict[str, str], allow_fail: bool = False, timeout: Optional[int] = None) -> None:
        """执行用户配置的命令（preBuild/postBuild），更安全的执行与日志"""
        if not commands:
            return

        self.logger.info("执行命令:")
        for cmd in commands:
            # 替换变量（如 ${PROJECT_NAME}）
            for key, value in env.items():
                cmd = cmd.replace(f"${{{key}}}", str(value))

            self.logger.info(f"> {cmd}")
            try:
                result = subprocess.run(
                    cmd,
                    shell=True,
                    check=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    cwd=str(self.project_root),
                    timeout=timeout
                )
                if result.stdout and self.verbose:
                    self.logger.debug(result.stdout.strip())
                if result.stderr:
                    self.logger.warning(f"命令 stderr: {result.stderr.strip()}")
            except subprocess.CalledProcessError as e:
                self.logger.error(f"命令失败 (returncode={e.returncode}): {cmd}")
                self.logger.error(e.stderr.strip() if e.stderr else str(e))
                if not allow_fail:
                    raise
            except subprocess.TimeoutExpired as e:
                self.logger.error(f"命令超时: {cmd}")
                if not allow_fail:
                    raise

    def _copy_resources(self, output_dir: Path) -> None:
        """复制用户配置的资源文件"""
        if not self.config.resources:
            return

        self.logger.info("复制资源文件:")
        for res in self.config.resources:
            src_pattern = res["src"]
            dest_dir = output_dir / res["dest"]
            dest_dir.mkdir(parents=True, exist_ok=True)

            # 查找匹配的源文件（支持 glob）
            src_files = list(self.project_root.glob(src_pattern))
            if not src_files:
                self.logger.warning(f"未找到资源: {src_pattern}")
                continue

            for src in src_files:
                try:
                    target = dest_dir / src.name
                    if src.is_dir():
                        shutil.copytree(src, target, dirs_exist_ok=True)
                    else:
                        shutil.copy2(src, target)
                    self.logger.info(f"→ {src} → {target}")
                except Exception as e:
                    self.logger.warning(f"复制资源失败: {src} -> {e}")

    def _build_nuitka_cmd(self, output_dir: Path, build_dir: Path, platform: str) -> List[str]:
        """生成 Nuitka 编译命令（核心）"""
        # 使用列表形式，外部不走 shell，避免空格问题
        cmd = [
            sys.executable, "-m", "nuitka",
            str(self.main_script),
            f"--output-dir={str(output_dir)}",
            f"--build-dir={str(build_dir)}",
        ]

        # 添加 Nuitka 配置参数
        nuitka = self.config.nuitka
        if nuitka.standalone:
            cmd.append("--standalone")
        if nuitka.follow_imports:
            cmd.append("--follow-imports")
        if nuitka.remove_output:
            cmd.append("--remove-output")
        if nuitka.show_progress:
            cmd.append("--show-progress")

        # 包/模块包含/排除
        for pkg in nuitka.include_packages:
            cmd.append(f"--include-package={pkg}")
        for mod in nuitka.include_modules:
            cmd.append(f"--include-module={mod}")
        for mod in nuitka.exclude_modules:
            cmd.append(f"--exclude-module={mod}")

        # 平台专属参数
        if platform.startswith("windows"):
            if nuitka.windows_icon:
                cmd.append(f"--windows-icon-from-ico={nuitka.windows_icon}")
            if nuitka.windows_company:
                cmd.append(f"--windows-company-name={nuitka.windows_company}")
            if nuitka.windows_product:
                cmd.append(f"--windows-product-name={nuitka.windows_product}")
        elif platform.startswith("macos") and nuitka.macos_app_name:
            cmd.append(f"--macos-app-name={nuitka.macos_app_name}")

        return cmd

    def _write_build_log(self, platform: str, content: str) -> Path:
        path = self.logs_dir / f"nuitka-{platform}-{datetime.utcnow().strftime('%Y%m%dT%H%M%SZ')}.log"
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        return path

    def _hash_source(self) -> str:
        """简单计算 src 目录的哈希（用于增量判断）"""
        h = hashlib.sha256()
        if not self.src_dir.exists():
            return ""
        for p in sorted(self.src_dir.rglob("*")):
            if p.is_file():
                h.update(str(p.relative_to(self.src_dir)).encode())
                try:
                    with open(p, "rb") as f:
                        h.update(hashlib.sha256(f.read()).digest())
                except Exception:
                    continue
        return h.hexdigest()

    def build(self, platform: Optional[str] = None, auto_increment: bool = False, force: bool = False) -> None:
        """构建项目（用户调用的核心方法）"""
        # 自动递增版本号
        if auto_increment:
            try:
                base, rev = self.config.version.rsplit(".", 1)
                self.config.version = f"{base}.{int(rev) + 1}"
            except Exception:
                self.config.version += ".1"
            self.logger.info(f"自动递增版本号: {self.config.version}")

        # 确定目标平台
        target_platforms = [platform] if platform else self.config.platforms

        # 计算源哈希用于增量构建
        src_hash = self._hash_source()
        cache_file = self.build_root / ".src_hash"
        prev_hash = None
        if cache_file.exists():
            try:
                prev_hash = cache_file.read_text(encoding="utf-8").strip()
            except Exception:
                prev_hash = None

        for plat in target_platforms:
            self.logger.info(f"\n===== 构建 {plat} 平台 =====")
            output_dir = self._get_platform_dir(plat)
            build_dir = self.build_root / plat

            # 增量判断
            if not force and prev_hash and prev_hash == src_hash and output_dir.exists():
                self.logger.info("检测到源码无变化且已存在构建产物，跳过构建（可用 --force 强制重建）")
                continue

            # 清理旧构建
            if build_dir.exists():
                shutil.rmtree(build_dir, ignore_errors=True)
            build_dir.mkdir(parents=True, exist_ok=True)
            output_dir.mkdir(parents=True, exist_ok=True)

            # 执行构建前命令
            self._run_commands(
                self.config.pre_build_commands,
                env={
                    "PROJECT_NAME": self.config.project_name,
                    "VERSION": self.config.version,
                    "OUTPUT_DIR": str(output_dir),
                    "PLATFORM": plat
                },
                allow_fail=False
            )

            # 执行 Nuitka 编译
            nuitka_cmd = self._build_nuitka_cmd(output_dir, build_dir, plat)
            self.logger.info(f"开始编译: {' '.join(nuitka_cmd[:5])} ...")  # 简化显示
            try:
                result = subprocess.run(
                    nuitka_cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    cwd=str(self.project_root)
                )
            except Exception as e:
                self.logger.error(f"调用 Nuitka 时发生异常: {e}")
                raise RuntimeError("构建失败：无法启动 Nuitka")

            # 写完整日志
            combined = ""
            if result.stdout:
                combined += "STDOUT:\n" + result.stdout + "\n"
            if result.stderr:
                combined += "STDERR:\n" + result.stderr + "\n"
            log_path = self._write_build_log(plat, combined)
            self.logger.debug(f"Nuitka 日志写入: {log_path}")

            if result.returncode != 0:
                # 打印简短摘要到控制台并保留完整日志文件
                err_snip = (result.stderr or "")[:1000]
                self.logger.error(f"编译失败（详情见日志）: {err_snip}")
                raise RuntimeError(f"构建失败，请检查日志: {log_path}")

            # 复制资源文件
            self._copy_resources(output_dir)

            # 执行构建后命令
            self._run_commands(
                self.config.post_build_commands,
                env={"OUTPUT_DIR": str(output_dir), "VERSION": self.config.version}
            )

            # 更新缓存哈希
            try:
                cache_file.parent.mkdir(parents=True, exist_ok=True)
                cache_file.write_text(src_hash, encoding="utf-8")
            except Exception:
                pass

            self.logger.info(f"===== 构建完成: {output_dir} =====")

    def clean(self, clean_all: bool = False) -> None:
        """清理构建产物"""
        if self.build_root.exists():
            shutil.rmtree(self.build_root, ignore_errors=True)
            self.logger.info(f"已清理临时构建目录: {self.build_root}")
        if clean_all and self.output_root.exists():
            shutil.rmtree(self.output_root, ignore_errors=True)
            self.logger.info(f"已清理输出目录: {self.output_root}")

    def package(self, platform: Optional[str] = None, out_name: Optional[str] = None) -> None:
        """打包构建结果为 ZIP"""
        plat = platform or (self.config.platforms[0] if self.config.platforms else "current")
        output_dir = self._get_platform_dir(plat)
        if not output_dir.exists():
            raise FileNotFoundError(f"未找到构建产物: {output_dir}，请先执行 build")

        zip_name = out_name or output_dir.name
        zip_path = self.output_root / zip_name
        shutil.make_archive(str(zip_path), "zip", root_dir=str(output_dir.parent), base_dir=output_dir.name)
        self.logger.info(f"已打包为: {zip_path}.zip")
