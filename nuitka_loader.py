"""
Nuitka 动态加载器
在运行时将打包的 Nuitka 副本加载到 sys.path
"""

import sys
from pathlib import Path


class NuitkaLoader:
    """管理打包的 Nuitka 副本"""
    
    @staticmethod
    def get_bundled_path() -> Path:
        """获取打包的 Nuitka 路径"""
        if getattr(sys, 'frozen', False):
            # 运行在打包后的 exe 中
            if hasattr(sys, '_MEIPASS'):
                # PyInstaller/Nuitka 打包
                base = Path(sys._MEIPASS)
            else:
                base = Path(sys.executable).parent
        else:
            # 开发模式
            base = Path(__file__).parent.parent / ".venv" / "Lib" / "site-packages"
        
        return base / "bundled_packages"
    
    @staticmethod
    def load_nuitka():
        """加载打包的 Nuitka 到 sys.path"""
        bundled_path = NuitkaLoader.get_bundled_path()
        
        if bundled_path.exists():
            # 将打包的 packages 目录添加到 sys.path 最前面
            bundled_str = str(bundled_path)
            if bundled_str not in sys.path:
                sys.path.insert(0, bundled_str)
                print(f"✓ 已加载打包的 Nuitka: {bundled_path}")
                return True
        
        # 回退：尝试使用系统已安装的 Nuitka
        try:
            import nuitka
            print(f"✓ 使用系统 Nuitka: {nuitka.__file__}")
            return True
        except ImportError:
            print("✗ 找不到 Nuitka")
            return False
    
    @staticmethod
    def ensure_nuitka():
        """确保 Nuitka 可用"""
        # 首先尝试加载打包的版本
        if NuitkaLoader.load_nuitka():
            return True
        
        # 如果都失败，提示用户安装
        print("=" * 70)
        print("❌ Nuitka 未找到！")
        print("\n请安装 Nuitka:")
        print("  pip install nuitka ordered-set zstandard")
        print("=" * 70)
        sys.exit(1)


# 模块导入时自动加载
NuitkaLoader.ensure_nuitka()
