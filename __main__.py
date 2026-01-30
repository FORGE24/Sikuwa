# sikuwa/__main__.py
"""
Sikuwa 入口点
"""

def main():
    """主入口函数 - 延迟导入避免循环依赖"""
    try:
        # 延迟导入，避免 mypyc 编译问题
        from sikuwa.cli import main as cli_main
        cli_main()
    except ImportError as e:
        print(f"[错误] 导入失败: {e}")
        print("请确保已安装所有依赖: pip install -r requirements.txt")
        import sys
        sys.exit(1)
    except Exception as e:
        print(f"[错误] 运行失败: {e}")
        import sys
        sys.exit(1)


if __name__ == "__main__":
    main()
