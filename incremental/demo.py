#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
减量编译演示 - Sikuwa Incremental Compilation Demo
展示"指哪编哪"的精准编译能力
"""

import tempfile
from pathlib import Path

from incremental import (
    IncrementalCompiler,
    PythonAnalyzer,
    BlockType
)


def demo_analyzer():
    """演示代码分析器"""
    print("=" * 60)
    print("1. 代码分析器演示")
    print("=" * 60)
    
    analyzer = PythonAnalyzer()
    
    code = '''
import os
from pathlib import Path

x = 10
y = 20

def add(a, b):
    """加法"""
    return a + b

def multiply(a, b):
    """乘法"""
    return a * b

class Calculator:
    """计算器类"""
    
    def __init__(self):
        self.history = []
    
    def calculate(self, op, a, b):
        if op == '+':
            result = add(a, b)
        elif op == '*':
            result = multiply(a, b)
        self.history.append(result)
        return result
'''
    
    blocks = analyzer.analyze(code, "demo.py")
    
    print(f"\n检测到 {len(blocks)} 个代码块:\n")
    
    for block in blocks:
        type_name = block.type.name.lower()
        deps = ', '.join(block.references[:5]) if block.references else '无'
        print(f"  [{type_name:10}] {block.name:20} 行 {block.start_line:2}-{block.end_line:2}  依赖: {deps}")


def demo_change_detection():
    """演示变更检测"""
    print("\n" + "=" * 60)
    print("2. 变更检测演示")
    print("=" * 60)
    
    with tempfile.TemporaryDirectory() as tmpdir:
        compiler = IncrementalCompiler(tmpdir)
        
        # 模拟编译器
        compile_count = [0]
        def mock_compile(unit):
            compile_count[0] += 1
            return f"COMPILED: {unit.name or 'unknown'}"
        
        compiler.set_compiler(mock_compile)
        
        # 初始代码
        code_v1 = '''
def hello():
    print("Hello")

def world():
    print("World")

def main():
    hello()
    world()
'''
        
        print("\n[v1] 初始代码:")
        compiler.analyze_source("demo.py", code_v1)
        changes = compiler.update_source("demo.py", code_v1)
        print(f"  检测到 {len(changes)} 个新增单元")
        
        outputs = compiler.compile_all_pending()
        print(f"  编译了 {compile_count[0]} 个单元")
        
        # 修改一个函数
        code_v2 = '''
def hello():
    print("Hello, World!")  # 修改了这行

def world():
    print("World")

def main():
    hello()
    world()
'''
        
        compile_count[0] = 0
        print("\n[v2] 修改 hello 函数:")
        changes = compiler.update_source("demo.py", code_v2)
        print(f"  检测到 {len(changes)} 个变更单元")
        for ch in changes:
            print(f"    - {ch.unit_id[:40]}... ({ch.change_type.name})")
        
        outputs = compiler.compile_all_pending()
        print(f"  只编译了 {compile_count[0]} 个单元 (其他使用缓存)")
        
        # 添加新函数
        code_v3 = '''
def hello():
    print("Hello, World!")

def world():
    print("World")

def greet(name):
    print(f"Hi, {name}!")

def main():
    hello()
    world()
    greet("Sikuwa")
'''
        
        compile_count[0] = 0
        print("\n[v3] 添加 greet 函数:")
        changes = compiler.update_source("demo.py", code_v3)
        print(f"  检测到 {len(changes)} 个变更单元")
        
        outputs = compiler.compile_all_pending()
        print(f"  编译了 {compile_count[0]} 个新/变更单元")
        
        # 统计
        stats = compiler.get_stats()
        print(f"\n统计: 缓存命中 {stats.get('cache_hits', 0)}, 总编译 {stats.get('total_compiled', 0)}")


def demo_dependency_tracking():
    """演示依赖追踪"""
    print("\n" + "=" * 60)
    print("3. 依赖追踪演示")
    print("=" * 60)
    
    with tempfile.TemporaryDirectory() as tmpdir:
        compiler = IncrementalCompiler(tmpdir)
        
        affected_units = []
        def mock_compile(unit):
            affected_units.append(unit.name or unit.id[:20])
            return f"COMPILED"
        
        compiler.set_compiler(mock_compile)
        
        code_v1 = '''
# 基础配置
CONFIG = {"debug": False}

def get_config():
    return CONFIG

def process():
    cfg = get_config()
    return cfg["debug"]

def main():
    result = process()
    print(result)
'''
        
        print("\n初始编译...")
        compiler.analyze_source("demo.py", code_v1)
        compiler.update_source("demo.py", code_v1)
        compiler.compile_all_pending()
        
        # 修改 CONFIG
        code_v2 = '''
# 基础配置
CONFIG = {"debug": True}  # 修改

def get_config():
    return CONFIG

def process():
    cfg = get_config()
    return cfg["debug"]

def main():
    result = process()
    print(result)
'''
        
        affected_units.clear()
        print("\n修改 CONFIG 后:")
        changes = compiler.update_source("demo.py", code_v2)
        
        # 显示依赖传播
        print("  受影响的单元链:")
        print("    CONFIG (修改) → get_config (依赖CONFIG) → process (依赖get_config)")
        
        compiler.compile_all_pending()
        print(f"  重新编译: {', '.join(affected_units) if affected_units else '无'}")


def demo_output_combination():
    """演示输出合并"""
    print("\n" + "=" * 60)
    print("4. 输出合并演示")
    print("=" * 60)
    
    with tempfile.TemporaryDirectory() as tmpdir:
        compiler = IncrementalCompiler(tmpdir)
        
        # 转换为 C 风格伪代码
        def to_pseudo_c(unit):
            lines = unit.content.strip().split('\n')
            result = []
            for line in lines:
                line = line.strip()
                if line.startswith('def '):
                    # def func(): -> void func() {
                    name = line[4:line.index('(')]
                    result.append(f"void {name}() {{")
                elif line.startswith('print('):
                    # print("x") -> printf("x");
                    content = line[6:-1]
                    result.append(f"    printf({content});")
                elif line == '':
                    continue
                else:
                    result.append(f"    // {line}")
            if result and not result[-1].endswith('}'):
                result.append("}")
            return '\n'.join(result)
        
        compiler.set_compiler(to_pseudo_c)
        
        code = '''
def hello():
    print("Hello")

def world():
    print("World")
'''
        
        compiler.analyze_source("demo.py", code)
        compiler.update_source("demo.py", code)
        compiler.compile_all_pending()
        
        combined = compiler.get_combined_output("demo.py")
        
        print("\n原始 Python 代码:")
        print(code)
        
        print("合并后的编译产物:")
        print(combined)


def main():
    """主函数"""
    print("\n" + "=" * 60)
    print("Sikuwa 减量编译系统演示")
    print("指哪编哪 - 精准编译，高效开发")
    print("=" * 60)
    
    demo_analyzer()
    demo_change_detection()
    demo_dependency_tracking()
    demo_output_combination()
    
    print("\n" + "=" * 60)
    print("演示完成!")
    print("=" * 60)


if __name__ == '__main__':
    main()
