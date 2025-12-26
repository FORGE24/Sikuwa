# sikuwa/incremental/__init__.py
"""
减量编译模块 - Incremental Compilation System
指哪编哪，精准编译

核心功能：
1. 单行/最小语法块为最小编译单元
2. 每个单元有唯一标识、最小依赖集、缓存产物
3. 版本快照对比检测变更
4. 只编译变更单元及受依赖影响的关联单元
5. 边界触发器处理函数/类
6. 按原始顺序拼接产物

智能缓存 V1.2：
- 编译即缓存：每次编译自动记录，全历史可追溯
- 缓存即编译：缓存命中等同于零成本编译
- 预测缓存预热：基于访问模式预测并预编译
"""

from .core import (
    IncrementalCompiler,
    CompilationUnit,
    Snapshot,
    ChangeRecord,
    ChangeDetector,
    CompilationCache,
    UnitType,
    UnitState,
)

from .analyzer import (
    PythonAnalyzer,
    CodeBlock,
    BlockType,
)

from .compiler_integration import (
    IncrementalNativeCompiler,
    IncrementalBuildResult,
    create_incremental_native_compiler,
)

from .smart_cache import (
    SmartCache,
    CacheEntry,
    CacheEvent,
    CacheEventType,
    get_smart_cache,
    create_smart_cache,
)

__all__ = [
    # 核心类
    'IncrementalCompiler',
    'CompilationUnit',
    'Snapshot',
    'ChangeRecord',
    'ChangeDetector',
    'CompilationCache',
    
    # 枚举
    'UnitType',
    'UnitState',
    
    # 分析器
    'PythonAnalyzer',
    'CodeBlock',
    'BlockType',
    
    # 集成编译器
    'IncrementalNativeCompiler',
    'IncrementalBuildResult',
    'create_incremental_native_compiler',
    
    # 智能缓存 V1.2
    'SmartCache',
    'CacheEntry',
    'CacheEvent',
    'CacheEventType',
    'get_smart_cache',
    'create_smart_cache',
]

__version__ = '1.2.0'
