# sikuwa/incremental/core.py
"""
减量编译核心 - Python 实现
指哪编哪：只编译源码改变的部分

提供 C++ 扩展不可用时的纯 Python 回退实现
"""

import hashlib
import json
import os
import time
from enum import Enum, auto
from dataclasses import dataclass, field
from typing import List, Dict, Set, Optional, Tuple, Callable, Any
from pathlib import Path

from .analyzer import PythonAnalyzer, CodeBlock, BlockType


class UnitType(Enum):
    """编译单元类型"""
    LINE = auto()
    STATEMENT = auto()
    FUNCTION = auto()
    CLASS = auto()
    MODULE = auto()
    IMPORT = auto()
    DECORATOR = auto()
    BLOCK = auto()


class UnitState(Enum):
    """编译单元状态"""
    UNKNOWN = auto()
    UNCHANGED = auto()
    MODIFIED = auto()
    ADDED = auto()
    DELETED = auto()
    AFFECTED = auto()


@dataclass
class CompilationUnit:
    """编译单元 - 最小编译粒度"""
    id: str = ""
    file_path: str = ""
    start_line: int = 0
    end_line: int = 0
    type: UnitType = UnitType.LINE
    name: str = ""
    content: str = ""
    content_hash: str = ""
    dependencies: List[str] = field(default_factory=list)
    dependents: List[str] = field(default_factory=list)
    state: UnitState = UnitState.UNKNOWN
    cached_output: str = ""
    cache_timestamp: int = 0
    cache_valid: bool = False
    
    def compute_hash(self) -> str:
        """计算内容哈希"""
        normalized = '\n'.join(line.strip() for line in self.content.splitlines())
        self.content_hash = hashlib.sha256(normalized.encode()).hexdigest()[:16]
        return self.content_hash
    
    def generate_id(self) -> str:
        """生成唯一ID"""
        if not self.content_hash:
            self.compute_hash()
        self.id = f"{self.file_path}:{self.start_line}:{self.end_line}:{self.content_hash[:8]}"
        return self.id
    
    @classmethod
    def from_code_block(cls, block: CodeBlock) -> 'CompilationUnit':
        """从 CodeBlock 创建"""
        unit = cls()
        unit.id = block.id
        unit.file_path = block.id.split(':')[0] if ':' in block.id else ""
        unit.start_line = block.start_line
        unit.end_line = block.end_line
        unit.content = block.content
        unit.content_hash = block.content_hash
        unit.name = block.name
        unit.dependencies = block.dependencies.copy()
        
        # 映射类型
        type_map = {
            BlockType.MODULE: UnitType.MODULE,
            BlockType.IMPORT: UnitType.IMPORT,
            BlockType.CLASS: UnitType.CLASS,
            BlockType.FUNCTION: UnitType.FUNCTION,
            BlockType.METHOD: UnitType.FUNCTION,
            BlockType.DECORATOR: UnitType.DECORATOR,
            BlockType.STATEMENT: UnitType.STATEMENT,
            BlockType.ASSIGNMENT: UnitType.STATEMENT,
            BlockType.EXPRESSION: UnitType.STATEMENT,
            BlockType.CONTROL: UnitType.BLOCK,
            BlockType.WITH: UnitType.BLOCK,
        }
        unit.type = type_map.get(block.type, UnitType.STATEMENT)
        
        return unit


@dataclass
class Snapshot:
    """版本快照"""
    file_path: str = ""
    content_hash: str = ""
    line_hashes: List[str] = field(default_factory=list)
    units: Dict[str, CompilationUnit] = field(default_factory=dict)
    timestamp: int = 0


@dataclass
class ChangeRecord:
    """变更记录"""
    unit_id: str = ""
    change_type: UnitState = UnitState.UNKNOWN
    old_start_line: int = 0
    old_end_line: int = 0
    new_start_line: int = 0
    new_end_line: int = 0
    reason: str = ""


class ChangeDetector:
    """变更检测器"""
    
    @staticmethod
    def compute_hash(content: str) -> str:
        """计算内容哈希"""
        return hashlib.sha256(content.encode()).hexdigest()[:16]
    
    @staticmethod
    def compute_line_hash(line: str) -> str:
        """计算行哈希（忽略首尾空白）"""
        stripped = line.strip()
        if not stripped:
            return "empty"
        return hashlib.sha256(stripped.encode()).hexdigest()[:16]
    
    def create_snapshot(self, file_path: str, content: str) -> Snapshot:
        """创建快照"""
        snap = Snapshot()
        snap.file_path = file_path
        snap.content_hash = self.compute_hash(content)
        snap.timestamp = int(time.time() * 1000)
        
        lines = content.splitlines()
        snap.line_hashes = [self.compute_line_hash(line) for line in lines]
        
        return snap
    
    def get_changed_lines(self, old_snap: Snapshot, new_snap: Snapshot) -> List[int]:
        """获取变更的行号 (1-based)"""
        # 使用 LCS 算法进行对比
        lcs = self._compute_lcs(old_snap.line_hashes, new_snap.line_hashes)
        
        # LCS 中新版本的行索引
        lcs_new_indices = {pair[1] for pair in lcs}
        
        # 不在 LCS 中的行即为变更的行
        changed = []
        for i in range(len(new_snap.line_hashes)):
            if i not in lcs_new_indices:
                changed.append(i + 1)  # 1-based
        
        return changed
    
    def _compute_lcs(self, old_hashes: List[str], new_hashes: List[str]) -> List[Tuple[int, int]]:
        """计算最长公共子序列"""
        m, n = len(old_hashes), len(new_hashes)
        
        # DP 表
        dp = [[0] * (n + 1) for _ in range(m + 1)]
        
        for i in range(1, m + 1):
            for j in range(1, n + 1):
                if old_hashes[i - 1] == new_hashes[j - 1]:
                    dp[i][j] = dp[i - 1][j - 1] + 1
                else:
                    dp[i][j] = max(dp[i - 1][j], dp[i][j - 1])
        
        # 回溯找出 LCS 对应关系
        lcs = []
        i, j = m, n
        while i > 0 and j > 0:
            if old_hashes[i - 1] == new_hashes[j - 1]:
                lcs.append((i - 1, j - 1))
                i -= 1
                j -= 1
            elif dp[i - 1][j] > dp[i][j - 1]:
                i -= 1
            else:
                j -= 1
        
        lcs.reverse()
        return lcs
    
    def detect_changes(self, old_snap: Snapshot, new_snap: Snapshot) -> List[ChangeRecord]:
        """检测变更"""
        records = []
        
        old_ids = set(old_snap.units.keys())
        new_ids = set(new_snap.units.keys())
        
        # 删除的单元
        for uid in old_ids - new_ids:
            old_unit = old_snap.units[uid]
            rec = ChangeRecord(
                unit_id=uid,
                change_type=UnitState.DELETED,
                old_start_line=old_unit.start_line,
                old_end_line=old_unit.end_line,
                reason="unit deleted"
            )
            records.append(rec)
        
        # 新增的单元
        for uid in new_ids - old_ids:
            new_unit = new_snap.units[uid]
            rec = ChangeRecord(
                unit_id=uid,
                change_type=UnitState.ADDED,
                new_start_line=new_unit.start_line,
                new_end_line=new_unit.end_line,
                reason="unit added"
            )
            records.append(rec)
        
        # 修改的单元
        for uid in old_ids & new_ids:
            old_unit = old_snap.units[uid]
            new_unit = new_snap.units[uid]
            if old_unit.content_hash != new_unit.content_hash:
                rec = ChangeRecord(
                    unit_id=uid,
                    change_type=UnitState.MODIFIED,
                    old_start_line=old_unit.start_line,
                    old_end_line=old_unit.end_line,
                    new_start_line=new_unit.start_line,
                    new_end_line=new_unit.end_line,
                    reason="content changed"
                )
                records.append(rec)
        
        return records


class CompilationCache:
    """
    编译缓存 V1.2
    
    编译即缓存，缓存即编译
    - 每次编译自动记录，全历史可追溯
    - 缓存命中等同于零成本编译
    - 集成预测预热
    """
    
    def __init__(self, cache_dir: str):
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        self._cache: Dict[str, Dict] = {}
        self._hits = 0
        self._misses = 0
        self._compile_history: List[Dict] = []  # 编译历史
        self._access_sequence: List[str] = []   # 访问序列
        self._predictions: Dict[str, List[str]] = {}  # 预测模式
        self._load()
    
    def _load(self):
        """加载缓存"""
        cache_file = self.cache_dir / "incremental_cache.json"
        history_file = self.cache_dir / "compile_history.json"
        patterns_file = self.cache_dir / "prediction_patterns.json"
        
        if cache_file.exists():
            try:
                with open(cache_file, 'r', encoding='utf-8') as f:
                    self._cache = json.load(f)
            except:
                self._cache = {}
        
        if history_file.exists():
            try:
                with open(history_file, 'r', encoding='utf-8') as f:
                    self._compile_history = json.load(f)
            except:
                self._compile_history = []
        
        if patterns_file.exists():
            try:
                with open(patterns_file, 'r', encoding='utf-8') as f:
                    self._predictions = json.load(f)
            except:
                self._predictions = {}
    
    def save(self):
        """保存缓存和历史"""
        cache_file = self.cache_dir / "incremental_cache.json"
        history_file = self.cache_dir / "compile_history.json"
        patterns_file = self.cache_dir / "prediction_patterns.json"
        
        with open(cache_file, 'w', encoding='utf-8') as f:
            json.dump(self._cache, f, indent=2)
        
        # 只保留最近10000条历史
        with open(history_file, 'w', encoding='utf-8') as f:
            json.dump(self._compile_history[-10000:], f, indent=2)
        
        with open(patterns_file, 'w', encoding='utf-8') as f:
            json.dump(self._predictions, f, indent=2)
    
    def has(self, unit_id: str) -> bool:
        return unit_id in self._cache
    
    def get(self, unit_id: str) -> str:
        """缓存即编译 - 命中即零成本获得编译结果"""
        if unit_id in self._cache:
            self._hits += 1
            # 记录访问序列
            self._record_access(unit_id)
            # 更新访问时间
            self._cache[unit_id]['last_access'] = int(time.time() * 1000)
            self._cache[unit_id]['access_count'] = self._cache[unit_id].get('access_count', 0) + 1
            return self._cache[unit_id].get('output', '')
        self._misses += 1
        return ""
    
    def put(self, unit_id: str, output: str, content_hash: str, 
            compile_time_ms: int = 0, file_path: str = "", 
            start_line: int = 0, end_line: int = 0):
        """编译即缓存 - 每次编译自动记录"""
        timestamp = int(time.time() * 1000)
        
        self._cache[unit_id] = {
            'output': output,
            'content_hash': content_hash,
            'timestamp': timestamp,
            'last_access': timestamp,
            'access_count': 1,
            'compile_time_ms': compile_time_ms,
            'file_path': file_path,
            'line_range': [start_line, end_line],
            'size_bytes': len(output.encode('utf-8')),
        }
        
        # 记录编译历史
        self._compile_history.append({
            'unit_id': unit_id,
            'content_hash': content_hash,
            'timestamp': timestamp,
            'compile_time_ms': compile_time_ms,
            'file_path': file_path,
            'action': 'compile'
        })
        
        # 记录访问序列
        self._record_access(unit_id)
    
    def _record_access(self, unit_id: str):
        """记录访问序列，用于预测"""
        # 更新访问序列
        self._access_sequence.append(unit_id)
        if len(self._access_sequence) > 1000:
            self._access_sequence = self._access_sequence[-500:]
        
        # 学习访问模式
        if len(self._access_sequence) >= 2:
            prev_id = self._access_sequence[-2]
            if prev_id != unit_id:
                if prev_id not in self._predictions:
                    self._predictions[prev_id] = []
                if unit_id not in self._predictions[prev_id]:
                    self._predictions[prev_id].append(unit_id)
                # 限制预测列表长度
                self._predictions[prev_id] = self._predictions[prev_id][:10]
    
    def get_predictions(self, unit_id: str) -> List[str]:
        """获取预测的下一个可能访问的单元"""
        return self._predictions.get(unit_id, [])
    
    def invalidate(self, unit_id: str):
        self._cache.pop(unit_id, None)
        # 记录失效历史
        self._compile_history.append({
            'unit_id': unit_id,
            'timestamp': int(time.time() * 1000),
            'action': 'invalidate'
        })
    
    def invalidate_all(self):
        self._cache.clear()
    
    def is_valid(self, unit_id: str, current_hash: str) -> bool:
        if unit_id not in self._cache:
            return False
        return self._cache[unit_id].get('content_hash') == current_hash
    
    def get_compile_history(self, limit: int = 100) -> List[Dict]:
        """获取编译历史"""
        return self._compile_history[-limit:]
    
    def get_hot_units(self, limit: int = 20) -> List[Dict]:
        """获取热点单元（访问最频繁）"""
        sorted_items = sorted(
            self._cache.items(),
            key=lambda x: x[1].get('access_count', 0),
            reverse=True
        )
        return [
            {'unit_id': k, 'access_count': v.get('access_count', 0),
             'file': v.get('file_path', ''), 'lines': v.get('line_range', [])}
            for k, v in sorted_items[:limit]
        ]
    
    def get_stats(self) -> Dict[str, Any]:
        """获取统计信息"""
        total_size = sum(e.get('size_bytes', 0) for e in self._cache.values())
        total_compile_time = sum(e.get('compile_time_ms', 0) for e in self._cache.values())
        return {
            'version': '1.2',
            'entries': len(self._cache),
            'total_size_mb': total_size / (1024 * 1024),
            'total_compile_time_ms': total_compile_time,
            'hits': self._hits,
            'misses': self._misses,
            'hit_rate': self._hits / (self._hits + self._misses) if (self._hits + self._misses) > 0 else 0,
            'history_count': len(self._compile_history),
            'prediction_patterns': len(self._predictions),
        }
    
    @property
    def hit_count(self) -> int:
        return self._hits
    
    @property
    def miss_count(self) -> int:
        return self._misses


class IncrementalCompiler:
    """
    减量编译器 - 指哪编哪
    
    核心功能：
    1. 以最小语法块为编译单元
    2. 变更检测 - 只定位修改的单元及受影响的关联单元
    3. 仅对变更单元重新编译，未变更单元复用缓存
    4. 边界触发器 - 自动扩展到函数/类边界
    5. 按原始顺序拼接产物
    """
    
    def __init__(self, cache_dir: str = ".sikuwa_cache"):
        self.cache = CompilationCache(cache_dir)
        self.detector = ChangeDetector()
        self.analyzer = PythonAnalyzer()
        
        self._units: Dict[str, CompilationUnit] = {}
        self._file_units: Dict[str, List[str]] = {}  # file -> unit_ids
        self._snapshots: Dict[str, Snapshot] = {}
        self._units_to_compile: List[str] = []
        
        # 编译器回调
        self._compile_callback: Optional[Callable[[CompilationUnit], str]] = None
    
    def set_compiler(self, callback: Callable[[CompilationUnit], str]):
        """设置编译器回调"""
        self._compile_callback = callback
    
    def analyze_source(self, file_path: str, content: str) -> List[CompilationUnit]:
        """分析源代码，返回编译单元列表"""
        blocks = self.analyzer.analyze(content, file_path)
        units = [CompilationUnit.from_code_block(b) for b in blocks]
        return units
    
    def register_units(self, file_path: str, units: List[CompilationUnit]):
        """注册编译单元"""
        # 移除旧单元
        if file_path in self._file_units:
            for uid in self._file_units[file_path]:
                self._units.pop(uid, None)
        
        # 添加新单元
        self._file_units[file_path] = []
        for unit in units:
            self._units[unit.id] = unit
            self._file_units[file_path].append(unit.id)
    
    def update_source(self, file_path: str, new_content: str) -> List[ChangeRecord]:
        """
        更新源代码并检测变更
        
        返回变更记录列表
        """
        # 分析新代码
        new_units = self.analyze_source(file_path, new_content)
        
        # 创建新快照
        new_snap = self.detector.create_snapshot(file_path, new_content)
        for unit in new_units:
            new_snap.units[unit.id] = unit
        
        changes = []
        self._units_to_compile = []
        
        # 检查是否有旧快照
        old_snap = self._snapshots.get(file_path)
        
        if old_snap:
            # 获取变更的行
            changed_lines = self.detector.get_changed_lines(old_snap, new_snap)
            
            # 找出受影响的编译单元
            affected_ids: Set[str] = set()
            
            for line in changed_lines:
                # 找出覆盖此行的单元
                for unit in new_units:
                    if unit.start_line <= line <= unit.end_line:
                        affected_ids.add(unit.id)
                        unit.state = UnitState.MODIFIED
                        unit.cache_valid = False
            
            # 传播依赖影响
            affected_ids = self._propagate_dependencies(affected_ids, new_units)
            
            # 扩展到边界
            affected_ids = self._expand_to_boundaries(affected_ids, new_units)
            
            # 生成变更记录
            for uid in affected_ids:
                unit = self._units.get(uid) or next((u for u in new_units if u.id == uid), None)
                if unit:
                    rec = ChangeRecord(
                        unit_id=uid,
                        change_type=unit.state if unit.state != UnitState.UNKNOWN else UnitState.MODIFIED,
                        new_start_line=unit.start_line,
                        new_end_line=unit.end_line,
                        reason="content changed"
                    )
                    changes.append(rec)
                    self._units_to_compile.append(uid)
        else:
            # 首次分析，所有单元都需要编译
            for unit in new_units:
                unit.state = UnitState.ADDED
                rec = ChangeRecord(
                    unit_id=unit.id,
                    change_type=UnitState.ADDED,
                    new_start_line=unit.start_line,
                    new_end_line=unit.end_line,
                    reason="first analysis"
                )
                changes.append(rec)
                self._units_to_compile.append(unit.id)
        
        # 注册单元并更新快照
        self.register_units(file_path, new_units)
        self._snapshots[file_path] = new_snap
        
        return changes
    
    def _propagate_dependencies(self, affected_ids: Set[str], 
                                units: List[CompilationUnit]) -> Set[str]:
        """传播依赖影响"""
        # 构建依赖图
        dependents: Dict[str, List[str]] = {}
        for unit in units:
            for dep_id in unit.dependencies:
                if dep_id not in dependents:
                    dependents[dep_id] = []
                dependents[dep_id].append(unit.id)
        
        # BFS 传播
        queue = list(affected_ids)
        visited = set(affected_ids)
        
        while queue:
            uid = queue.pop(0)
            for dependent_id in dependents.get(uid, []):
                if dependent_id not in visited:
                    visited.add(dependent_id)
                    queue.append(dependent_id)
                    # 标记为受影响
                    for unit in units:
                        if unit.id == dependent_id:
                            unit.state = UnitState.AFFECTED
                            unit.cache_valid = False
                            break
        
        return visited
    
    def _expand_to_boundaries(self, affected_ids: Set[str],
                              units: List[CompilationUnit]) -> Set[str]:
        """扩展到函数/类边界"""
        expanded = set(affected_ids)
        unit_map = {u.id: u for u in units}
        
        for uid in list(affected_ids):
            unit = unit_map.get(uid)
            if not unit:
                continue
            
            # 如果在函数/类内部修改，需要重新编译整个结构
            for other in units:
                if other.id == uid:
                    continue
                # 检查是否被包含
                if (other.type in (UnitType.FUNCTION, UnitType.CLASS) and
                    other.start_line <= unit.start_line and
                    other.end_line >= unit.end_line):
                    expanded.add(other.id)
                    other.state = UnitState.AFFECTED
                    other.cache_valid = False
        
        return expanded
    
    def get_units_to_compile(self) -> List[str]:
        """获取需要编译的单元ID列表"""
        return self._units_to_compile.copy()
    
    def compile_unit(self, unit_id: str) -> str:
        """
        编译单个单元
        
        缓存即编译：缓存命中 = 零成本获得编译结果
        """
        unit = self._units.get(unit_id)
        if not unit:
            return ""
        
        # 检查缓存 - 缓存即编译
        if unit.cache_valid or self.cache.is_valid(unit_id, unit.content_hash):
            output = self.cache.get(unit_id)
            if output:
                unit.cached_output = output
                unit.cache_valid = True
                # 触发预测预热
                self._predictive_warmup(unit_id)
                return output
        
        # 执行编译并计时
        start_time = time.time()
        if self._compile_callback:
            output = self._compile_callback(unit)
        else:
            # 默认：直接返回源代码（用于测试）
            output = unit.content
        compile_time_ms = int((time.time() - start_time) * 1000)
        
        # 编译即缓存：自动记录
        self.mark_compiled(unit_id, output, compile_time_ms)
        
        return output
    
    def _predictive_warmup(self, unit_id: str):
        """预测性缓存预热"""
        # 获取预测的下一个访问单元
        predictions = self.cache.get_predictions(unit_id)
        for pred_id in predictions[:2]:  # 最多预热2个
            if pred_id in self._units and not self.cache.has(pred_id):
                # 加入待编译队列
                if pred_id not in self._units_to_compile:
                    self._units_to_compile.append(pred_id)
    
    def mark_compiled(self, unit_id: str, output: str, compile_time_ms: int = 0):
        """标记单元编译完成 - 编译即缓存"""
        unit = self._units.get(unit_id)
        if unit:
            unit.cached_output = output
            unit.cache_timestamp = int(time.time() * 1000)
            unit.cache_valid = True
            unit.state = UnitState.UNCHANGED
            
            # 编译即缓存：记录完整信息
            self.cache.put(
                unit_id, output, unit.content_hash,
                compile_time_ms=compile_time_ms,
                file_path=unit.file_path,
                start_line=unit.start_line,
                end_line=unit.end_line
            )
        
        # 从待编译列表移除
        if unit_id in self._units_to_compile:
            self._units_to_compile.remove(unit_id)
    
    def compile_all_pending(self) -> Dict[str, str]:
        """编译所有待编译单元"""
        results = {}
        for uid in self._units_to_compile.copy():
            output = self.compile_unit(uid)
            results[uid] = output
        return results
    
    def get_combined_output(self, file_path: str) -> str:
        """获取合并后的编译输出（按原始顺序拼接）"""
        if file_path not in self._file_units:
            return ""
        
        # 按行号排序
        unit_ids = self._file_units[file_path]
        units = [self._units[uid] for uid in unit_ids if uid in self._units]
        units.sort(key=lambda u: u.start_line)
        
        # 拼接输出
        outputs = []
        for unit in units:
            output = unit.cached_output
            if not output and self.cache.has(unit.id):
                output = self.cache.get(unit.id)
            if output:
                outputs.append(output)
        
        return '\n'.join(outputs)
    
    def get_stats(self) -> Dict[str, Any]:
        """获取统计信息"""
        cache_stats = self.cache.get_stats()
        return {
            'total_units': len(self._units),
            'pending_units': len(self._units_to_compile),
            'files': len(self._file_units),
            **cache_stats,  # 包含缓存详细统计
        }
    
    def get_compile_history(self, limit: int = 100) -> List[Dict]:
        """获取编译历史"""
        return self.cache.get_compile_history(limit)
    
    def get_hot_units(self, limit: int = 20) -> List[Dict]:
        """获取热点单元"""
        return self.cache.get_hot_units(limit)
    
    def get_predictions(self, unit_id: str) -> List[str]:
        """获取预测的下一个访问单元"""
        return self.cache.get_predictions(unit_id)
    
    def save(self):
        """保存状态"""
        self.cache.save()
    
    def clear(self):
        """清空所有状态"""
        self._units.clear()
        self._file_units.clear()
        self._snapshots.clear()
        self._units_to_compile.clear()
        self.cache.invalidate_all()


# 尝试导入 C++ 扩展
_cpp_available = False
try:
    from .cpp import incremental_engine as _cpp_engine
    _cpp_available = True
except ImportError:
    pass


def create_incremental_compiler(cache_dir: str = ".sikuwa_cache", 
                                prefer_cpp: bool = True) -> IncrementalCompiler:
    """
    创建减量编译器实例
    
    Args:
        cache_dir: 缓存目录
        prefer_cpp: 是否优先使用 C++ 实现
        
    Returns:
        IncrementalCompiler 实例
    """
    # 目前返回 Python 实现
    # TODO: 当 C++ 扩展可用时，返回包装器
    return IncrementalCompiler(cache_dir)
