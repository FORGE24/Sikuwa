# sikuwa/cpp_cache/__init__.py
# Python包装器模块，用于使用C++实现的智能缓存系统

import os
import sys
import json
import hashlib
from pathlib import Path

# 尝试导入C++扩展模块
try:
    from .pysmartcache import (
        lru_cache_new,
        lru_cache_contains,
        lru_cache_put,
        lru_cache_get,
        lru_cache_remove,
        lru_cache_clear
    )
    cpp_extension_loaded = True
except ImportError as e:
    print(f"Warning: pysmartcache C++ extension not found. Using fallback implementation. Error: {e}")
    cpp_extension_loaded = False

# Python回退实现
if not cpp_extension_loaded:
    class FallbackLRUCache:
        """纯Python实现的LRU缓存"""
        
        def __init__(self, max_size=1000):
            self.max_size = max_size
            self.cache = {}
            self.usage_order = []
        
        def contains(self, key):
            return key in self.cache
        
        def put(self, key, value):
            if key in self.cache:
                # 移动到最近使用
                self.usage_order.remove(key)
            elif len(self.cache) >= self.max_size:
                # 移除最久未使用的
                oldest = self.usage_order.pop(0)
                del self.cache[oldest]
            
            self.cache[key] = value
            self.usage_order.append(key)
            return True
        
        def get(self, key):
            if key not in self.cache:
                return ""
            
            # 移动到最近使用
            self.usage_order.remove(key)
            self.usage_order.append(key)
            return self.cache[key]
        
        def remove(self, key):
            if key in self.cache:
                del self.cache[key]
                self.usage_order.remove(key)
                return True
            return False
        
        def clear(self):
            self.cache.clear()
            self.usage_order.clear()
            return True
    
    # 模拟C++扩展的函数
    def lru_cache_new(max_size=1000):
        return FallbackLRUCache(max_size)
    
    def lru_cache_contains(cache, key):
        return cache.contains(key)
    
    def lru_cache_put(cache, key, value):
        return cache.put(key, value)
    
    def lru_cache_get(cache, key):
        return cache.get(key)
    
    def lru_cache_remove(cache, key):
        return cache.remove(key)
    
    def lru_cache_clear(cache):
        return cache.clear()

# LRUCache类的Python包装器
class LRUCache:
    """LRU (Least Recently Used) 缓存的Python包装器"""
    
    def __init__(self, max_size=1000):
        """创建一个新的LRU缓存"""
        self.cache = lru_cache_new(max_size)
    
    def contains(self, key):
        """检查缓存中是否包含指定的键"""
        return lru_cache_contains(self.cache, key)
    
    def put(self, key, value):
        """将键值对放入缓存"""
        return lru_cache_put(self.cache, key, value)
    
    def get(self, key):
        """从缓存中获取指定键的值"""
        return lru_cache_get(self.cache, key)
    
    def remove(self, key):
        """从缓存中移除指定的键"""
        return lru_cache_remove(self.cache, key)
    
    def clear(self):
        """清空缓存"""
        lru_cache_clear(self.cache)

# 纯Python实现的BuildCache
class BuildCache:
    """构建缓存系统"""
    
    def __init__(self, cache_dir=".cache", max_size=1000000000):
        """创建一个新的构建缓存"""
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.max_size = max_size
        self.cache_file = self.cache_dir / "build_cache.json"
        self.cache = self._load_cache()
    
    def _load_cache(self):
        """从文件加载缓存"""
        if self.cache_file.exists():
            try:
                with open(self.cache_file, 'r', encoding='utf-8') as f:
                    return json.load(f)
            except Exception:
                pass
        return {}
    
    def _save_cache(self):
        """保存缓存到文件"""
        try:
            with open(self.cache_file, 'w', encoding='utf-8') as f:
                json.dump(self.cache, f, indent=2, ensure_ascii=False)
        except Exception as e:
            print(f"Error saving cache: {e}")
    
    def set_cache_strategy(self, strategy):
        """设置缓存策略 ("lru" 或 "lfu")"""
        # Python实现不支持策略切换，这里只是为了兼容接口
        pass
    
    def cache_build_result(self, target, command, dependencies, result):
        """缓存构建结果"""
        cache_key = self._generate_cache_key(target, command, dependencies)
        self.cache[cache_key] = {
            "result": result,
            "dependencies": dependencies,
            "command": command,
            "timestamp": os.path.getmtime(__file__)
        }
        self._save_cache()
        return True
    
    def get_cached_build_result(self, target, command, dependencies):
        """获取缓存的构建结果"""
        cache_key = self._generate_cache_key(target, command, dependencies)
        if cache_key in self.cache:
            return self.cache[cache_key]["result"]
        return ""
    
    def needs_rebuild(self, target, command, dependencies):
        """检查是否需要重新构建"""
        cache_key = self._generate_cache_key(target, command, dependencies)
        if cache_key not in self.cache:
            return True
        
        # 这里简单地总是返回False，因为我们已经生成了包含所有依赖的缓存键
        # 在实际实现中，可以检查依赖文件是否有变化
        return False
    
    def _generate_cache_key(self, target, command, dependencies):
        """生成缓存键"""
        # 合并所有信息生成唯一的缓存键
        all_info = f"{target}|{command}|{json.dumps(dependencies, sort_keys=True)}"
        return hashlib.sha256(all_info.encode()).hexdigest()
    
    def clean_all_cache(self):
        """清理所有缓存"""
        self.cache.clear()
        self._save_cache()
        return True
    
    def dump_stats(self):
        """打印缓存统计信息"""
        print(f"Build Cache Statistics:")
        print(f"  Cache directory: {self.cache_dir}")
        print(f"  Number of cached items: {len(self.cache)}")

# 为BuildCache创建类似C++扩展的函数接口
def build_cache_new(cache_dir=".cache", max_size=1000000000):
    return BuildCache(cache_dir, max_size)

def build_cache_set_cache_strategy(cache, strategy):
    return cache.set_cache_strategy(strategy)

def build_cache_cache_build_result(cache, target, command, dependencies, result):
    # 如果dependencies是字符串，将其转换为列表
    if isinstance(dependencies, str):
        dependencies = [dependencies]
    return cache.cache_build_result(target, command, dependencies, result)

def build_cache_get_cached_build_result(cache, target, command, dependencies):
    # 如果dependencies是字符串，将其转换为列表
    if isinstance(dependencies, str):
        dependencies = [dependencies]
    return cache.get_cached_build_result(target, command, dependencies)

def build_cache_needs_rebuild(cache, target, command, dependencies):
    # 如果dependencies是字符串，将其转换为列表
    if isinstance(dependencies, str):
        dependencies = [dependencies]
    return cache.needs_rebuild(target, command, dependencies)

def build_cache_clean_all_cache(cache):
    return cache.clean_all_cache()

def build_cache_dump_build_cache_stats(cache):
    return cache.dump_stats()

# 导出所有函数
globals().update({
    'build_cache_new': build_cache_new,
    'build_cache_set_cache_strategy': build_cache_set_cache_strategy,
    'build_cache_cache_build_result': build_cache_cache_build_result,
    'build_cache_get_cached_build_result': build_cache_get_cached_build_result,
    'build_cache_needs_rebuild': build_cache_needs_rebuild,
    'build_cache_clean_all_cache': build_cache_clean_all_cache,
    'build_cache_dump_build_cache_stats': build_cache_dump_build_cache_stats
})
