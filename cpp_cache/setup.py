# sikuwa/cpp_cache/setup.py
# 用于编译和安装C++智能缓存扩展模块的setup文件

from setuptools import setup, Extension
import sys

# 获取Python的include目录
py_include_dirs = [sys.prefix + '/include']

# 定义扩展模块
smart_cache_extension = Extension(
    'pysmartcache',  # 扩展模块名称
    sources=['smart_cache_minimal.cpp', 'pysmartcache_minimal.cpp'],  # 源文件
    include_dirs=[".", *py_include_dirs],  # 包含目录
    language='c++',  # 使用C++
    extra_compile_args=['/STD:c++17'],  # 编译参数
)

# 设置setup配置
setup(
    name='sikuwa_cpp_cache',
    version='0.1',
    description='C++ Smart Cache System for Sikuwa',
    author='Sikuwa Team',
    author_email='',
    packages=['sikuwa.cpp_cache'],
    package_dir={'sikuwa.cpp_cache': '.'},
    ext_modules=[smart_cache_extension],
    zip_safe=False,
)
