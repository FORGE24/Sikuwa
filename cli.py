# cli.py
import argparse
import sys
from .parser import SkiParser
from .builder import SikuwaBuilder

def main():
    # 解析用户输入的命令
    parser = argparse.ArgumentParser(
        prog="sikuwa",
        description="Sikuwa 构建工具 - 通过 build.ski 配置项目构建"
    )
    parser.add_argument("command", nargs="?", default="help",
                        help="命令: build (构建), clean (清理), package (打包), help (帮助)")
    parser.add_argument("--platform", help="指定平台 (windows/macos/linux/current)")
    parser.add_argument("--auto-increment", action="store_true",
                        help="构建时自动递增版本号")
    parser.add_argument("--clean-all", action="store_true",
                        help="clean 时同时删除输出目录")
    parser.add_argument("--force", action="store_true",
                        help="强制重新构建（忽略增量缓存）")
    parser.add_argument("--verbose", action="store_true",
                        help="输出详细日志 (DEBUG)")
    args = parser.parse_args()

    try:
        # 解析用户的 build.ski 配置
        config_parser = SkiParser()
        builder = SikuwaBuilder(config_parser.config, verbose=args.verbose)
    except Exception as e:
        print(f"错误: {str(e)}", file=sys.stderr)
        return

    # 执行用户命令
    try:
        if args.command == "build":
            builder.build(args.platform, args.auto_increment, force=args.force)
        elif args.command == "clean":
            builder.clean(args.clean_all)
        elif args.command == "package":
            builder.package(args.platform)
        elif args.command == "help":
            print("""sikuwa 构建工具使用帮助:

命令:
  build        按 build.ski 配置构建项目
  clean        清理构建临时文件
  package      将构建结果打包为 ZIP
  help         显示此帮助信息

选项:
  --platform <平台>      指定构建平台 (如 windows/macos/linux，默认 current)
  --auto-increment       构建时自动递增版本号 (如 1.0.0 → 1.0.1)
  --clean-all            清理时同时删除输出目录 (dist)
  --force                强制重构建（忽略缓存）
  --verbose              输出详细日志

示例:
  sikuwa build               # 构建当前平台
  sikuwa build --platform windows  # 构建 Windows 平台
  sikuwa build --auto-increment --force    # 构建并递增版本，强制重建
  sikuwa clean --clean-all         # 彻底清理所有产物
        """)
        else:
            print(f"未知命令: {args.command}，使用 'sikuwa help' 查看帮助")
    except Exception as e:
        print(f"执行失败: {e}", file=sys.stderr)

if __name__ == "__main__":
    main()
