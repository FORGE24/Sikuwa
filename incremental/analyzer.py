# sikuwa/incremental/analyzer.py
"""
Python 代码分析器 - 识别代码块边界和依赖关系
用于减量编译的 AST 分析
"""

import ast
import hashlib
from enum import Enum, auto
from dataclasses import dataclass, field
from typing import List, Dict, Set, Optional, Tuple
from pathlib import Path


class BlockType(Enum):
    """代码块类型"""
    MODULE = auto()      # 模块级
    IMPORT = auto()      # 导入语句
    CLASS = auto()       # 类定义
    FUNCTION = auto()    # 函数定义
    METHOD = auto()      # 方法定义
    DECORATOR = auto()   # 装饰器
    STATEMENT = auto()   # 普通语句
    ASSIGNMENT = auto()  # 赋值语句
    EXPRESSION = auto()  # 表达式
    CONTROL = auto()     # 控制流 (if/for/while/try)
    WITH = auto()        # with 语句


@dataclass
class CodeBlock:
    """代码块 - 最小编译单元"""
    id: str = ""                    # 唯一标识
    type: BlockType = BlockType.STATEMENT
    name: str = ""                  # 名称（函数名/类名等）
    start_line: int = 0             # 起始行 (1-based)
    end_line: int = 0               # 结束行 (1-based)
    start_col: int = 0              # 起始列
    end_col: int = 0                # 结束列
    content: str = ""               # 源代码内容
    content_hash: str = ""          # 内容哈希
    parent_id: str = ""             # 父块ID
    children: List[str] = field(default_factory=list)  # 子块ID列表
    
    # 依赖信息
    imports: List[str] = field(default_factory=list)      # 导入的模块/名称
    references: List[str] = field(default_factory=list)   # 引用的名称
    definitions: List[str] = field(default_factory=list)  # 定义的名称
    dependencies: List[str] = field(default_factory=list) # 依赖的块ID
    
    def compute_hash(self) -> str:
        """计算内容哈希"""
        # 去除空白差异的影响
        normalized = '\n'.join(line.strip() for line in self.content.splitlines())
        self.content_hash = hashlib.sha256(normalized.encode()).hexdigest()[:16]
        return self.content_hash
    
    def generate_id(self, file_path: str) -> str:
        """生成唯一ID"""
        if not self.content_hash:
            self.compute_hash()
        self.id = f"{file_path}:{self.start_line}:{self.end_line}:{self.content_hash[:8]}"
        return self.id


class PythonAnalyzer:
    """
    Python 代码分析器
    分析代码结构，识别编译单元边界和依赖关系
    """
    
    def __init__(self):
        self.blocks: List[CodeBlock] = []
        self.block_map: Dict[str, CodeBlock] = {}
        self.lines: List[str] = []
        self.file_path: str = ""
        
    def analyze(self, source: str, file_path: str = "<string>") -> List[CodeBlock]:
        """
        分析 Python 源代码，返回代码块列表
        
        Args:
            source: Python 源代码
            file_path: 文件路径
            
        Returns:
            代码块列表
        """
        self.file_path = file_path
        self.lines = source.splitlines()
        self.blocks = []
        self.block_map = {}
        
        try:
            tree = ast.parse(source)
            self._analyze_module(tree, source)
        except SyntaxError as e:
            # 语法错误时回退到行级分析
            self._fallback_line_analysis(source)
        
        # 分析依赖关系
        self._analyze_dependencies()
        
        return self.blocks
    
    def _analyze_module(self, tree: ast.Module, source: str):
        """分析模块级 AST"""
        for node in ast.iter_child_nodes(tree):
            block = self._node_to_block(node, source)
            if block:
                self.blocks.append(block)
                self.block_map[block.id] = block
    
    def _node_to_block(self, node: ast.AST, source: str, parent_id: str = "") -> Optional[CodeBlock]:
        """将 AST 节点转换为代码块"""
        block = CodeBlock()
        block.parent_id = parent_id
        
        # 获取行号范围
        block.start_line = getattr(node, 'lineno', 0)
        block.end_line = getattr(node, 'end_lineno', block.start_line)
        block.start_col = getattr(node, 'col_offset', 0)
        block.end_col = getattr(node, 'end_col_offset', 0)
        
        # 提取源代码内容
        if block.start_line > 0 and block.end_line > 0:
            block.content = self._get_source_lines(block.start_line, block.end_line)
        
        # 根据节点类型设置块类型和名称
        if isinstance(node, ast.Import):
            block.type = BlockType.IMPORT
            block.name = "import"
            block.imports = [alias.name for alias in node.names]
            
        elif isinstance(node, ast.ImportFrom):
            block.type = BlockType.IMPORT
            block.name = f"from {node.module}"
            block.imports = [node.module or ""] + [alias.name for alias in node.names]
            
        elif isinstance(node, ast.ClassDef):
            block.type = BlockType.CLASS
            block.name = node.name
            block.definitions = [node.name]
            # 处理装饰器
            if node.decorator_list:
                block.start_line = node.decorator_list[0].lineno
            # 递归处理类体
            for child in node.body:
                child_block = self._node_to_block(child, source, block.id)
                if child_block:
                    block.children.append(child_block.id)
                    self.blocks.append(child_block)
                    self.block_map[child_block.id] = child_block
                    
        elif isinstance(node, ast.FunctionDef) or isinstance(node, ast.AsyncFunctionDef):
            block.type = BlockType.FUNCTION if not parent_id else BlockType.METHOD
            block.name = node.name
            block.definitions = [node.name]
            # 处理装饰器
            if node.decorator_list:
                block.start_line = node.decorator_list[0].lineno
            # 分析函数体中的引用
            block.references = self._extract_references(node)
            
        elif isinstance(node, ast.Assign):
            block.type = BlockType.ASSIGNMENT
            block.definitions = self._extract_targets(node.targets)
            block.references = self._extract_references(node.value)
            
        elif isinstance(node, ast.AugAssign):
            block.type = BlockType.ASSIGNMENT
            block.definitions = self._extract_targets([node.target])
            block.references = self._extract_references(node.value)
            
        elif isinstance(node, ast.AnnAssign):
            block.type = BlockType.ASSIGNMENT
            if node.target:
                block.definitions = self._extract_targets([node.target])
            if node.value:
                block.references = self._extract_references(node.value)
                
        elif isinstance(node, (ast.If, ast.For, ast.While, ast.Try)):
            block.type = BlockType.CONTROL
            block.name = node.__class__.__name__.lower()
            block.references = self._extract_references(node)
            
        elif isinstance(node, ast.With):
            block.type = BlockType.WITH
            block.references = self._extract_references(node)
            
        elif isinstance(node, ast.Expr):
            block.type = BlockType.EXPRESSION
            block.references = self._extract_references(node.value)
            
        else:
            block.type = BlockType.STATEMENT
            block.references = self._extract_references(node)
        
        # 计算哈希并生成ID
        block.compute_hash()
        block.generate_id(self.file_path)
        
        return block
    
    def _get_source_lines(self, start: int, end: int) -> str:
        """获取指定行范围的源代码"""
        if start < 1 or end > len(self.lines):
            return ""
        return '\n'.join(self.lines[start-1:end])
    
    def _extract_references(self, node: ast.AST) -> List[str]:
        """提取节点中引用的名称"""
        refs = []
        for child in ast.walk(node):
            if isinstance(child, ast.Name):
                refs.append(child.id)
            elif isinstance(child, ast.Attribute):
                # 收集属性链的根名称
                current = child
                while isinstance(current, ast.Attribute):
                    current = current.value
                if isinstance(current, ast.Name):
                    refs.append(current.id)
        return list(set(refs))
    
    def _extract_targets(self, targets: List[ast.AST]) -> List[str]:
        """提取赋值目标的名称"""
        names = []
        for target in targets:
            if isinstance(target, ast.Name):
                names.append(target.id)
            elif isinstance(target, ast.Tuple) or isinstance(target, ast.List):
                for elt in target.elts:
                    if isinstance(elt, ast.Name):
                        names.append(elt.id)
        return names
    
    def _analyze_dependencies(self):
        """分析块之间的依赖关系"""
        # 构建名称到块的映射
        name_to_block: Dict[str, str] = {}
        for block in self.blocks:
            for name in block.definitions:
                name_to_block[name] = block.id
        
        # 分析每个块的依赖
        for block in self.blocks:
            for ref in block.references:
                if ref in name_to_block and name_to_block[ref] != block.id:
                    dep_id = name_to_block[ref]
                    if dep_id not in block.dependencies:
                        block.dependencies.append(dep_id)
    
    def _fallback_line_analysis(self, source: str):
        """回退到行级分析（用于语法错误的代码）"""
        lines = source.splitlines()
        current_block = None
        indent_stack = [(0, None)]  # (indent, block)
        
        for i, line in enumerate(lines, 1):
            stripped = line.lstrip()
            if not stripped or stripped.startswith('#'):
                continue
                
            indent = len(line) - len(stripped)
            
            # 简单的块检测
            if stripped.startswith('def ') or stripped.startswith('async def '):
                block = CodeBlock(
                    type=BlockType.FUNCTION,
                    name=stripped.split('(')[0].replace('def ', '').replace('async ', '').strip(),
                    start_line=i,
                    end_line=i,
                    content=line
                )
                current_block = block
                
            elif stripped.startswith('class '):
                block = CodeBlock(
                    type=BlockType.CLASS,
                    name=stripped.split('(')[0].split(':')[0].replace('class ', '').strip(),
                    start_line=i,
                    end_line=i,
                    content=line
                )
                current_block = block
                
            elif stripped.startswith('import ') or stripped.startswith('from '):
                block = CodeBlock(
                    type=BlockType.IMPORT,
                    start_line=i,
                    end_line=i,
                    content=line
                )
                block.compute_hash()
                block.generate_id(self.file_path)
                self.blocks.append(block)
                self.block_map[block.id] = block
                continue
            
            else:
                if current_block and indent > indent_stack[-1][0]:
                    # 继续当前块
                    current_block.end_line = i
                    current_block.content += '\n' + line
                else:
                    # 结束当前块
                    if current_block:
                        current_block.compute_hash()
                        current_block.generate_id(self.file_path)
                        self.blocks.append(current_block)
                        self.block_map[current_block.id] = current_block
                        current_block = None
                    
                    # 普通语句
                    block = CodeBlock(
                        type=BlockType.STATEMENT,
                        start_line=i,
                        end_line=i,
                        content=line
                    )
                    block.compute_hash()
                    block.generate_id(self.file_path)
                    self.blocks.append(block)
                    self.block_map[block.id] = block
        
        # 处理最后一个块
        if current_block:
            current_block.compute_hash()
            current_block.generate_id(self.file_path)
            self.blocks.append(current_block)
            self.block_map[current_block.id] = current_block
    
    def get_blocks_in_range(self, start_line: int, end_line: int) -> List[CodeBlock]:
        """获取指定行范围内的代码块"""
        result = []
        for block in self.blocks:
            # 检查是否有交集
            if block.start_line <= end_line and block.end_line >= start_line:
                result.append(block)
        return result
    
    def get_affected_blocks(self, changed_block_ids: Set[str]) -> Set[str]:
        """获取受变更影响的所有块（包括依赖传播）"""
        affected = set(changed_block_ids)
        queue = list(changed_block_ids)
        
        while queue:
            block_id = queue.pop(0)
            # 找出依赖此块的所有块
            for block in self.blocks:
                if block_id in block.dependencies and block.id not in affected:
                    affected.add(block.id)
                    queue.append(block.id)
        
        return affected
    
    def expand_to_boundaries(self, block_ids: Set[str]) -> Set[str]:
        """扩展块ID集合，确保完整结构被包含"""
        expanded = set(block_ids)
        
        for block_id in list(block_ids):
            block = self.block_map.get(block_id)
            if not block:
                continue
            
            # 如果块在某个函数/类内，需要重新编译整个结构
            if block.parent_id:
                parent = self.block_map.get(block.parent_id)
                if parent and parent.type in (BlockType.CLASS, BlockType.FUNCTION):
                    expanded.add(parent.id)
                    # 也包含所有子块
                    for child_id in parent.children:
                        expanded.add(child_id)
            
            # 如果块是函数/类，包含所有子块
            if block.type in (BlockType.CLASS, BlockType.FUNCTION):
                for child_id in block.children:
                    expanded.add(child_id)
        
        return expanded


def analyze_python_file(file_path: str) -> List[CodeBlock]:
    """分析 Python 文件"""
    with open(file_path, 'r', encoding='utf-8') as f:
        source = f.read()
    
    analyzer = PythonAnalyzer()
    return analyzer.analyze(source, file_path)


def analyze_python_source(source: str, file_path: str = "<string>") -> List[CodeBlock]:
    """分析 Python 源代码"""
    analyzer = PythonAnalyzer()
    return analyzer.analyze(source, file_path)
