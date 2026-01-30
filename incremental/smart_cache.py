# sikuwa/incremental/smart_cache.py
"""
智能缓存系统 V1.2
编译即缓存，缓存即编译，预测缓存预热

深度集成减量编译引擎，实现：
1. 编译即缓存 - 每次编译自动持久化，全历史可追溯
2. 缓存即编译 - 缓存命中等同于零成本编译
3. 预测缓存预热 - 基于访问模式和依赖图预测并预编译
"""

import hashlib
import json
import os
import time
import threading
import queue
from enum import Enum, auto
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Set, Optional, Tuple, Callable, Any
from pathlib import Path
from collections import OrderedDict


class CacheEventType(Enum):
    """缓存事件类型"""
    HIT = auto()        # 命中
    MISS = auto()       # 未命中
    WRITE = auto()      # 写入
    EVICT = auto()      # 淘汰
    WARMUP = auto()     # 预热
    PREDICT = auto()    # 预测


@dataclass
class CacheEntry:
    """缓存条目"""
    key: str = ""
    content_hash: str = ""
    output: str = ""
    timestamp: int = 0
    access_count: int = 0
    last_access: int = 0
    dependencies: List[str] = field(default_factory=list)
    file_path: str = ""
    line_range: Tuple[int, int] = (0, 0)
    compile_time_ms: int = 0
    size_bytes: int = 0
    
    def touch(self):
        """更新访问信息"""
        self.access_count += 1
        self.last_access = int(time.time() * 1000)
    
    def to_dict(self) -> dict:
        return {
            'key': self.key,
            'content_hash': self.content_hash,
            'output': self.output,
            'timestamp': self.timestamp,
            'access_count': self.access_count,
            'last_access': self.last_access,
            'dependencies': self.dependencies,
            'file_path': self.file_path,
            'line_range': list(self.line_range),
            'compile_time_ms': self.compile_time_ms,
            'size_bytes': self.size_bytes,
        }
    
    @classmethod
    def from_dict(cls, data: dict) -> 'CacheEntry':
        entry = cls()
        entry.key = data.get('key', '')
        entry.content_hash = data.get('content_hash', '')
        entry.output = data.get('output', '')
        entry.timestamp = data.get('timestamp', 0)
        entry.access_count = data.get('access_count', 0)
        entry.last_access = data.get('last_access', 0)
        entry.dependencies = data.get('dependencies', [])
        entry.file_path = data.get('file_path', '')
        line_range = data.get('line_range', [0, 0])
        entry.line_range = tuple(line_range) if isinstance(line_range, list) else line_range
        entry.compile_time_ms = data.get('compile_time_ms', 0)
        entry.size_bytes = data.get('size_bytes', 0)
        return entry


@dataclass
class CacheEvent:
    """缓存事件记录"""
    event_type: CacheEventType
    key: str
    timestamp: int
    details: str = ""


@dataclass
class AccessPattern:
    """访问模式记录"""
    key: str
    access_sequence: List[str] = field(default_factory=list)  # 之后访问的键
    frequency: int = 0
    
    def record_next(self, next_key: str):
        """记录后续访问"""
        if next_key not in self.access_sequence:
            self.access_sequence.append(next_key)
        self.frequency += 1


class SmartCache:
    """
    智能缓存系统 V1.2
    
    核心特性：
    - LRU 淘汰策略 + 访问频率权重
    - 全历史编译记录持久化
    - 基于访问模式的预测预热
    - 依赖图感知的缓存失效
    - 后台异步预热线程
    """
    
    def __init__(self, 
                 cache_dir: str = ".sikuwa_cache",
                 max_entries: int = 10000,
                 max_size_mb: int = 500,
                 enable_warmup: bool = True):
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        
        self.max_entries = max_entries
        self.max_size_bytes = max_size_mb * 1024 * 1024
        self.enable_warmup = enable_warmup
        
        # 主缓存存储 (LRU)
        self._cache: OrderedDict[str, CacheEntry] = OrderedDict()
        self._total_size = 0
        
        # 统计信息
        self._hits = 0
        self._misses = 0
        self._evictions = 0
        self._warmups = 0
        
        # 事件日志
        self._events: List[CacheEvent] = []
        self._max_events = 10000
        
        # 访问模式追踪
        self._last_accessed_key: Optional[str] = None
        self._access_patterns: Dict[str, AccessPattern] = {}
        
        # 编译器回调（用于预热）
        self._compiler_callback: Optional[Callable] = None
        
        # 预热队列和线程
        self._warmup_queue: queue.Queue = queue.Queue()
        self._warmup_thread: Optional[threading.Thread] = None
        self._warmup_running = False
        
        # 加载持久化数据
        self._load()
        
        # 启动预热线程
        if enable_warmup:
            self._start_warmup_thread()
    
    def _load(self):
        """加载持久化缓存"""
        cache_file = self.cache_dir / "smart_cache_v1.2.json"
        patterns_file = self.cache_dir / "access_patterns.json"
        
        if cache_file.exists():
            try:
                with open(cache_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                for entry_data in data.get('entries', []):
                    entry = CacheEntry.from_dict(entry_data)
                    self._cache[entry.key] = entry
                    self._total_size += entry.size_bytes
            except Exception:
                pass
        
        if patterns_file.exists():
            try:
                with open(patterns_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                for key, pattern_data in data.items():
                    self._access_patterns[key] = AccessPattern(
                        key=key,
                        access_sequence=pattern_data.get('sequence', []),
                        frequency=pattern_data.get('frequency', 0)
                    )
            except Exception:
                pass
    
    def save(self):
        """保存缓存到磁盘"""
        cache_file = self.cache_dir / "smart_cache_v1.2.json"
        patterns_file = self.cache_dir / "access_patterns.json"
        events_file = self.cache_dir / "cache_events.json"
        
        # 保存缓存条目
        with open(cache_file, 'w', encoding='utf-8') as f:
            json.dump({
                'version': '1.2',
                'entries': [entry.to_dict() for entry in self._cache.values()]
            }, f, indent=2)
        
        # 保存访问模式
        with open(patterns_file, 'w', encoding='utf-8') as f:
            patterns = {
                k: {'sequence': p.access_sequence, 'frequency': p.frequency}
                for k, p in self._access_patterns.items()
            }
            json.dump(patterns, f, indent=2)
        
        # 保存事件日志（最近的）
        with open(events_file, 'w', encoding='utf-8') as f:
            events = [
                {'type': e.event_type.name, 'key': e.key, 
                 'timestamp': e.timestamp, 'details': e.details}
                for e in self._events[-1000:]  # 只保存最近1000条
            ]
            json.dump(events, f, indent=2)
    
    def set_compiler(self, callback: Callable):
        """设置编译器回调（用于预热编译）"""
        self._compiler_callback = callback
    
    # ==================== 核心缓存操作 ====================
    
    def get(self, key: str, content_hash: str = "") -> Optional[str]:
        """
        获取缓存 - 缓存即编译
        
        缓存命中 = 零成本获得编译结果
        """
        if key in self._cache:
            entry = self._cache[key]
            
            # 验证内容哈希（如果提供）
            if content_hash and entry.content_hash != content_hash:
                self._record_event(CacheEventType.MISS, key, "hash mismatch")
                self._misses += 1
                return None
            
            # 命中：移到末尾（LRU）
            self._cache.move_to_end(key)
            entry.touch()
            
            self._record_event(CacheEventType.HIT, key)
            self._hits += 1
            
            # 记录访问模式
            self._record_access_pattern(key)
            
            # 触发预测预热
            if self.enable_warmup:
                self._trigger_predictive_warmup(key)
            
            return entry.output
        
        self._record_event(CacheEventType.MISS, key)
        self._misses += 1
        return None
    
    def put(self, key: str, output: str, content_hash: str, 
            dependencies: List[str] = None,
            file_path: str = "",
            line_range: Tuple[int, int] = (0, 0),
            compile_time_ms: int = 0) -> bool:
        """
        写入缓存 - 编译即缓存
        
        每次编译结果自动持久化，全历史可追溯
        """
        size_bytes = len(output.encode('utf-8'))
        
        # 检查是否需要淘汰
        while (len(self._cache) >= self.max_entries or 
               self._total_size + size_bytes > self.max_size_bytes):
            if not self._evict_one():
                break
        
        # 创建或更新条目
        entry = CacheEntry(
            key=key,
            content_hash=content_hash,
            output=output,
            timestamp=int(time.time() * 1000),
            access_count=1,
            last_access=int(time.time() * 1000),
            dependencies=dependencies or [],
            file_path=file_path,
            line_range=line_range,
            compile_time_ms=compile_time_ms,
            size_bytes=size_bytes,
        )
        
        # 更新旧条目的大小
        if key in self._cache:
            self._total_size -= self._cache[key].size_bytes
        
        self._cache[key] = entry
        self._total_size += size_bytes
        
        self._record_event(CacheEventType.WRITE, key, 
                          f"size={size_bytes}, compile_time={compile_time_ms}ms")
        
        # 记录访问模式
        self._record_access_pattern(key)
        
        return True
    
    def invalidate(self, key: str):
        """使单个缓存失效"""
        if key in self._cache:
            self._total_size -= self._cache[key].size_bytes
            del self._cache[key]
            self._record_event(CacheEventType.EVICT, key, "manual invalidate")
    
    def invalidate_by_dependency(self, dep_key: str):
        """使所有依赖指定键的缓存失效"""
        to_invalidate = []
        for key, entry in self._cache.items():
            if dep_key in entry.dependencies:
                to_invalidate.append(key)
        
        for key in to_invalidate:
            self.invalidate(key)
    
    def _evict_one(self) -> bool:
        """淘汰一个条目（LRU + 频率权重）"""
        if not self._cache:
            return False
        
        # 计算淘汰分数（越低越优先淘汰）
        # 分数 = access_count * 0.3 + recency_score * 0.7
        now = int(time.time() * 1000)
        min_score = float('inf')
        evict_key = None
        
        for key, entry in self._cache.items():
            recency = (now - entry.last_access) / 1000  # 秒
            score = entry.access_count * 0.3 - recency * 0.001
            if score < min_score:
                min_score = score
                evict_key = key
        
        if evict_key:
            self._total_size -= self._cache[evict_key].size_bytes
            del self._cache[evict_key]
            self._evictions += 1
            self._record_event(CacheEventType.EVICT, evict_key, "LRU eviction")
            return True
        
        return False
    
    # ==================== 访问模式追踪 ====================
    
    def _record_access_pattern(self, key: str):
        """记录访问模式"""
        if self._last_accessed_key and self._last_accessed_key != key:
            if self._last_accessed_key not in self._access_patterns:
                self._access_patterns[self._last_accessed_key] = AccessPattern(
                    key=self._last_accessed_key
                )
            self._access_patterns[self._last_accessed_key].record_next(key)
        
        self._last_accessed_key = key
    
    # ==================== 预测缓存预热 ====================
    
    def _start_warmup_thread(self):
        """启动后台预热线程"""
        if self._warmup_thread and self._warmup_thread.is_alive():
            return
        
        self._warmup_running = True
        self._warmup_thread = threading.Thread(target=self._warmup_worker, daemon=True)
        self._warmup_thread.start()
    
    def _warmup_worker(self):
        """预热工作线程"""
        while self._warmup_running:
            try:
                # 等待预热任务
                task = self._warmup_queue.get(timeout=1.0)
                if task is None:
                    continue
                
                key, content, content_hash = task
                
                # 检查是否已缓存
                if key in self._cache:
                    continue
                
                # 执行预热编译
                if self._compiler_callback:
                    try:
                        start = time.time()
                        output = self._compiler_callback(content)
                        compile_time = int((time.time() - start) * 1000)
                        
                        self.put(key, output, content_hash, 
                                compile_time_ms=compile_time)
                        self._warmups += 1
                        self._record_event(CacheEventType.WARMUP, key,
                                          f"predictive warmup, time={compile_time}ms")
                    except Exception:
                        pass
                
            except queue.Empty:
                continue
    
    def _trigger_predictive_warmup(self, key: str):
        """触发预测性预热"""
        if key not in self._access_patterns:
            return
        
        pattern = self._access_patterns[key]
        
        # 预热接下来可能访问的键
        for next_key in pattern.access_sequence[:3]:  # 最多预热3个
            if next_key not in self._cache:
                self._record_event(CacheEventType.PREDICT, next_key,
                                  f"predicted from {key}")
                # 这里只是标记预测，实际预热需要内容
                # 真正的预热在 warmup_unit 中执行
    
    def warmup_unit(self, key: str, content: str, content_hash: str):
        """手动添加预热任务"""
        if key not in self._cache:
            self._warmup_queue.put((key, content, content_hash))
    
    def warmup_dependencies(self, keys: List[str], 
                           content_provider: Callable[[str], Tuple[str, str]]):
        """
        预热依赖链
        
        content_provider: key -> (content, content_hash)
        """
        for key in keys:
            if key not in self._cache:
                try:
                    content, content_hash = content_provider(key)
                    self._warmup_queue.put((key, content, content_hash))
                except Exception:
                    pass
    
    def stop_warmup(self):
        """停止预热线程"""
        self._warmup_running = False
        if self._warmup_thread:
            self._warmup_thread.join(timeout=2.0)
    
    # ==================== 事件日志 ====================
    
    def _record_event(self, event_type: CacheEventType, key: str, details: str = ""):
        """记录缓存事件"""
        event = CacheEvent(
            event_type=event_type,
            key=key,
            timestamp=int(time.time() * 1000),
            details=details
        )
        self._events.append(event)
        
        # 限制事件数量
        if len(self._events) > self._max_events:
            self._events = self._events[-self._max_events//2:]
    
    def get_recent_events(self, count: int = 100) -> List[dict]:
        """获取最近的事件"""
        return [
            {'type': e.event_type.name, 'key': e.key,
             'timestamp': e.timestamp, 'details': e.details}
            for e in self._events[-count:]
        ]
    
    # ==================== 统计和诊断 ====================
    
    def get_stats(self) -> Dict[str, Any]:
        """获取缓存统计"""
        return {
            'version': '1.2',
            'entries': len(self._cache),
            'total_size_mb': self._total_size / (1024 * 1024),
            'max_entries': self.max_entries,
            'max_size_mb': self.max_size_bytes / (1024 * 1024),
            'hits': self._hits,
            'misses': self._misses,
            'hit_rate': self._hits / (self._hits + self._misses) if (self._hits + self._misses) > 0 else 0,
            'evictions': self._evictions,
            'warmups': self._warmups,
            'access_patterns': len(self._access_patterns),
        }
    
    def get_hot_entries(self, count: int = 10) -> List[Dict]:
        """获取最热门的缓存条目"""
        sorted_entries = sorted(
            self._cache.values(),
            key=lambda e: e.access_count,
            reverse=True
        )
        return [
            {'key': e.key, 'access_count': e.access_count, 
             'file': e.file_path, 'lines': e.line_range}
            for e in sorted_entries[:count]
        ]
    
    def get_predicted_next(self, key: str, count: int = 5) -> List[str]:
        """获取预测的下一个访问键"""
        if key not in self._access_patterns:
            return []
        return self._access_patterns[key].access_sequence[:count]
    
    def has(self, key: str) -> bool:
        """检查键是否存在"""
        return key in self._cache
    
    def clear(self):
        """清空缓存"""
        self._cache.clear()
        self._total_size = 0
        self._access_patterns.clear()
        self._events.clear()
    
    def __del__(self):
        """析构时停止预热线程并保存"""
        self.stop_warmup()
        try:
            self.save()
        except Exception:
            pass


# ==================== 工厂函数 ====================

_global_cache: Optional[SmartCache] = None

def get_smart_cache(cache_dir: str = ".sikuwa_cache") -> SmartCache:
    """获取全局智能缓存实例"""
    global _global_cache
    if _global_cache is None:
        _global_cache = SmartCache(cache_dir)
    return _global_cache


def create_smart_cache(cache_dir: str = ".sikuwa_cache",
                       max_entries: int = 10000,
                       max_size_mb: int = 500,
                       enable_warmup: bool = True) -> SmartCache:
    """创建新的智能缓存实例"""
    return SmartCache(cache_dir, max_entries, max_size_mb, enable_warmup)
