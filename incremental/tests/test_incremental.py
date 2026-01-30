# sikuwa/incremental/tests/test_incremental.py
"""
减量编译系统测试
"""

import sys
import os
import tempfile
import unittest
from pathlib import Path

# 添加父目录到路径
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from incremental.core import (
    IncrementalCompiler,
    CompilationUnit,
    Snapshot,
    ChangeDetector,
    CompilationCache,
    UnitType,
    UnitState
)
from incremental.analyzer import PythonAnalyzer, BlockType


class TestPythonAnalyzer(unittest.TestCase):
    """测试 Python 分析器"""
    
    def setUp(self):
        self.analyzer = PythonAnalyzer()
    
    def test_analyze_function(self):
        """测试函数分析"""
        code = '''
def hello(name):
    """Say hello"""
    print(f"Hello, {name}!")
'''
        blocks = self.analyzer.analyze(code, "test.py")
        
        # 应该检测到函数块
        func_blocks = [b for b in blocks if b.type == BlockType.FUNCTION]
        self.assertEqual(len(func_blocks), 1)
        self.assertEqual(func_blocks[0].name, "hello")
    
    def test_analyze_class(self):
        """测试类分析"""
        code = '''
class MyClass:
    def __init__(self):
        self.value = 0
    
    def increment(self):
        self.value += 1
'''
        blocks = self.analyzer.analyze(code, "test.py")
        
        # 应该检测到类块
        class_blocks = [b for b in blocks if b.type == BlockType.CLASS]
        self.assertEqual(len(class_blocks), 1)
        self.assertEqual(class_blocks[0].name, "MyClass")
    
    def test_analyze_import(self):
        """测试导入分析"""
        code = '''
import os
from sys import path
from pathlib import Path
'''
        blocks = self.analyzer.analyze(code, "test.py")
        
        import_blocks = [b for b in blocks if b.type == BlockType.IMPORT]
        self.assertEqual(len(import_blocks), 3)
    
    def test_dependency_extraction(self):
        """测试依赖提取"""
        code = '''
def outer():
    def inner():
        return x
    return inner()
'''
        blocks = self.analyzer.analyze(code, "test.py")
        func_blocks = [b for b in blocks if b.type == BlockType.FUNCTION]
        
        # outer 函数应该依赖 x
        self.assertEqual(len(func_blocks), 1)
        self.assertIn('x', func_blocks[0].references)


class TestChangeDetector(unittest.TestCase):
    """测试变更检测器"""
    
    def setUp(self):
        self.detector = ChangeDetector()
    
    def test_detect_addition(self):
        """测试新增检测"""
        old = Snapshot()
        old.units = {}
        
        new_unit = CompilationUnit(
            id="u1", content="def foo(): pass",
            start_line=1, end_line=1, file_path="test.py"
        )
        new_unit.compute_hash()
        
        new = Snapshot()
        new.units = {"u1": new_unit}
        
        changes = self.detector.detect_changes(old, new)
        
        self.assertEqual(len(changes), 1)
        self.assertEqual(changes[0].unit_id, "u1")
        self.assertEqual(changes[0].change_type, UnitState.ADDED)
    
    def test_detect_modification(self):
        """测试修改检测"""
        old_unit = CompilationUnit(
            id="u1", content="def foo(): pass",
            start_line=1, end_line=1, file_path="test.py"
        )
        old_unit.compute_hash()
        
        old = Snapshot()
        old.units = {"u1": old_unit}
        
        new_unit = CompilationUnit(
            id="u1", content="def foo(): return 1",
            start_line=1, end_line=1, file_path="test.py"
        )
        new_unit.compute_hash()
        
        new = Snapshot()
        new.units = {"u1": new_unit}
        
        changes = self.detector.detect_changes(old, new)
        
        self.assertEqual(len(changes), 1)
        self.assertEqual(changes[0].unit_id, "u1")
        self.assertEqual(changes[0].change_type, UnitState.MODIFIED)
    
    def test_detect_deletion(self):
        """测试删除检测"""
        old_unit = CompilationUnit(
            id="u1", content="def foo(): pass",
            start_line=1, end_line=1, file_path="test.py"
        )
        old_unit.compute_hash()
        
        old = Snapshot()
        old.units = {"u1": old_unit}
        
        new = Snapshot()
        new.units = {}
        
        changes = self.detector.detect_changes(old, new)
        
        self.assertEqual(len(changes), 1)
        self.assertEqual(changes[0].change_type, UnitState.DELETED)


class TestCompilationCache(unittest.TestCase):
    """测试编译缓存"""
    
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.cache = CompilationCache(self.temp_dir)
    
    def tearDown(self):
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)
    
    def test_put_get(self):
        """测试缓存存取"""
        self.cache.put("key1", "value1", "hash1")
        result = self.cache.get("key1")
        self.assertEqual(result, "value1")
    
    def test_get_nonexistent(self):
        """测试获取不存在的键"""
        result = self.cache.get("nonexistent")
        self.assertEqual(result, "")  # 返回空字符串
    
    def test_persistence(self):
        """测试持久化"""
        self.cache.put("key1", "value1", "hash1")
        self.cache.save()
        
        # 创建新缓存实例
        cache2 = CompilationCache(self.temp_dir)
        result = cache2.get("key1")
        self.assertEqual(result, "value1")


class TestIncrementalCompiler(unittest.TestCase):
    """测试减量编译器"""
    
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.compiler = IncrementalCompiler(self.temp_dir)
        
        # 设置简单的编译器（返回大写代码）
        self.compiler.set_compiler(lambda unit: unit.content.upper())
    
    def tearDown(self):
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)
    
    def test_initial_compile(self):
        """测试初始编译"""
        code = '''
def hello():
    print("Hello")

def world():
    print("World")
'''
        self.compiler.analyze_source("test.py", code)
        changes = self.compiler.update_source("test.py", code)
        
        # 首次编译，所有单元都应该是新的
        self.assertGreater(len(changes), 0)
        
        # 编译
        outputs = self.compiler.compile_all_pending()
        self.assertGreater(len(outputs), 0)
    
    def test_incremental_compile(self):
        """测试增量编译"""
        # 初始代码
        code1 = '''
def hello():
    print("Hello")

def world():
    print("World")
'''
        self.compiler.analyze_source("test.py", code1)
        self.compiler.update_source("test.py", code1)
        outputs1 = self.compiler.compile_all_pending()
        
        # 修改一个函数
        code2 = '''
def hello():
    print("Hello Modified")

def world():
    print("World")
'''
        changes = self.compiler.update_source("test.py", code2)
        
        # 应该有变更
        self.assertGreater(len(changes), 0)
        
        # 再次编译
        outputs2 = self.compiler.compile_all_pending()
        
        # 验证有输出
        self.assertGreater(len(outputs1) + len(outputs2), 0)
    
    def test_dependency_propagation(self):
        """测试依赖传播"""
        code = '''
x = 10

def get_x():
    return x

def double_x():
    return get_x() * 2
'''
        self.compiler.analyze_source("test.py", code)
        self.compiler.update_source("test.py", code)
        self.compiler.compile_all_pending()
        
        # 修改 x 的值
        code2 = '''
x = 20

def get_x():
    return x

def double_x():
    return get_x() * 2
'''
        changes = self.compiler.update_source("test.py", code2)
        
        # 应该检测到变更（x 变了，依赖它的也应该被标记）
        self.assertGreater(len(changes), 0)
    
    def test_combined_output(self):
        """测试合并输出"""
        code = '''
import os

def hello():
    print("Hello")

def world():
    print("World")
'''
        self.compiler.analyze_source("test.py", code)
        self.compiler.update_source("test.py", code)
        self.compiler.compile_all_pending()
        
        combined = self.compiler.get_combined_output("test.py")
        
        # 合并输出应该包含所有编译产物
        self.assertGreater(len(combined), 0)


class TestBlockBoundary(unittest.TestCase):
    """测试边界触发器"""
    
    def setUp(self):
        self.analyzer = PythonAnalyzer()
    
    def test_class_contains_methods(self):
        """测试类包含其方法"""
        code = '''
class MyClass:
    def method1(self):
        pass
    
    def method2(self):
        pass
'''
        blocks = self.analyzer.analyze(code, "test.py")
        
        class_blocks = [b for b in blocks if b.type == BlockType.CLASS]
        self.assertEqual(len(class_blocks), 1)
        
        # 类块应该包含整个类定义
        class_block = class_blocks[0]
        self.assertIn("method1", class_block.content)
        self.assertIn("method2", class_block.content)


def run_tests():
    """运行所有测试"""
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    
    suite.addTests(loader.loadTestsFromTestCase(TestPythonAnalyzer))
    suite.addTests(loader.loadTestsFromTestCase(TestChangeDetector))
    suite.addTests(loader.loadTestsFromTestCase(TestCompilationCache))
    suite.addTests(loader.loadTestsFromTestCase(TestIncrementalCompiler))
    suite.addTests(loader.loadTestsFromTestCase(TestBlockBoundary))
    
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    return result.wasSuccessful()


if __name__ == '__main__':
    success = run_tests()
    sys.exit(0 if success else 1)
