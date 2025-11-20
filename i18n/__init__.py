import gettext
import os
from pathlib import Path

# 获取翻译文件目录
i18n_dir = Path(__file__).parent
locale_dir = i18n_dir / "locales"

# 初始化翻译系统
translation = gettext.translation(
    "sikuwa", 
    localedir=str(locale_dir),  # 使用字符串路径
    languages=None,  # 使用系统默认语言
    fallback=True    # 如果找不到翻译文件，使用原始字符串
)

# 导出翻译函数
_ = translation.gettext

# 提供切换语言的功能
def set_language(lang_code):
    """切换当前使用的语言"""
    global translation, _
    try:
        translation = gettext.translation(
            "sikuwa", 
            localedir=str(locale_dir),  # 使用字符串路径
            languages=[lang_code],
            fallback=True
        )
        _ = translation.gettext
        return True
    except Exception as e:
        return False