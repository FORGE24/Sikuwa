# fix.py
"""修复 builder.py 的缩进问题"""
def fix_builder():
    with open('sikuwa/builder.py', 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    fixed_lines = []
    in_write_build_log = False
    
    for i, line in enumerate(lines):
        # 检测到 _write_build_log 方法定义
        if 'def _write_build_log(self' in line and not line.startswith('    def'):
            # 修复缩进：添加 4 个空格
            fixed_lines.append('    ' + line.lstrip())
            in_write_build_log = True
        
        # 该方法内部的代码也需要修复缩进
        elif in_write_build_log:
            # 如果遇到下一个方法定义或类定义，结束修复
            if line.strip() and not line.startswith(' ') and ('def ' in line or 'class ' in line):
                in_write_build_log = False
                fixed_lines.append(line)
            # 如果是该方法内部的代码，添加 4 个空格
            elif line.strip():
                fixed_lines.append('    ' + line)
            else:
                fixed_lines.append(line)
        
        # 修复 build_all 的 bug
        elif "summary.get('failed', 1)" in line:
            fixed_lines.append(line.replace("get('failed', 1)", "get('failed', 0)"))
        
        # 其他行保持不变
        else:
            fixed_lines.append(line)
    
    # 写回文件
    with open('sikuwa/builder.py', 'w', encoding='utf-8') as f:
        f.writelines(fixed_lines)
    
    print("✅ builder.py 已修复")
    print("请重新运行: sikuwa build -v")
if __name__ == '__main__':
    fix_builder()