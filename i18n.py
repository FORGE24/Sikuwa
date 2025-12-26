# sikuwa/i18n.py
"""
国际化(i18n)支持模块
"""

import os
import gettext
from pathlib import Path
from typing import Optional

# 支持的语言列表
supported_languages = ['zh_CN', 'en_US']

# 默认语言
default_language = 'zh_CN'

# 翻译目录
LOCALES_DIR = Path(__file__).parent / 'i18n' / 'locales'

# 当前翻译对象
trans = None

def setup_i18n(force_lang: Optional[str] = None):
    """
    初始化国际化支持（添加调试功能）
    
    Args:
        force_lang: 强制使用的语言，如 'en_US' 或 'zh_CN'，用于调试
    """
    global trans
    
    # 验证语言是否支持
    if force_lang not in supported_languages:
        force_lang = default_language
    
    # 设置环境变量
    os.environ['LANGUAGE'] = force_lang
    
    # 创建翻译对象
    trans = gettext.translation(
        domain='sikuwa',
        localedir=LOCALES_DIR,
        languages=[force_lang],
        fallback=True
    )
    
    # 安装翻译函数
    trans.install()
    
    # 返回选择的语言，便于调试时确认当前使用的语言
    return force_lang

# 初始化翻译（默认使用系统语言）
selected_lang = setup_i18n()

# 调试：打印当前使用的语言
print(f"[i18n调试] 当前使用的语言: {selected_lang}")

# 导出_函数供其他模块使用
_ = trans.gettext


def test_translation():
    """
    测试国际化是否正常工作的调试函数
    """
    print("[i18n调试] 开始测试翻译...")
    test_strings = [
        _("Hello, world!"),
        _("Welcome to Sikuwa"),
        _("Settings"),
        _("Exit")
    ]
    for idx, s in enumerate(test_strings, 1):
        print(f"[i18n调试] 翻译测试 {idx}: {s}")
    print("[i18n调试] 翻译测试结束")

