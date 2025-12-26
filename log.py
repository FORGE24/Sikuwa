# sikuwa/log.py
"""
Sikuwa 超详细日志系统
支持 34 级日志等级，用于精确追踪程序执行
"""

import logging
import sys
import time
import functools
from pathlib import Path
from typing import Optional, Any, Callable
from datetime import datetime
from enum import IntEnum

# 兼容扁平结构和包结构的导入
try:
    from sikuwa.i18n import _
except ImportError:
    from i18n import _


class LogLevel(IntEnum):
    """34 级日志等级"""
    # TRACE 级别 (1-5)
    TRACE_IO = 1          # 极细粒度 I/O 跟踪
    TRACE_STATE = 2       # 极细粒度状态变更
    TRACE_PERF = 3        # 微观性能计时
    TRACE_FLOW = 4        # 函数进入退出跟踪
    TRACE_MSG = 5         # 消息队列/事件传递
    
    # DEBUG 级别 (6-10)
    DEBUG_DETAIL = 6      # 详细调试信息
    DEBUG_CONFIG = 7      # 配置/启动参数
    DEBUG_CONN = 8        # 连接建立/断开
    DEBUG_CACHE = 9       # 缓存命中/失效
    DEBUG_SQL = 10        # SQL/查询执行
    
    # INFO 级别 (11-15)
    INFO_OPERATION = 11   # 业务操作记录
    INFO_USER = 12        # 用户可见操作
    INFO_METRIC = 13      # 周期性指标快照
    INFO_DEPLOY = 14      # 部署/升级事件
    INFO_HEALTH = 15      # 健康检查通过
    
    # NOTICE 级别 (16-18)
    NOTICE_CONFIG = 16    # 非致命配置变更
    NOTICE_POLICY = 17    # 策略/权限变更
    NOTICE_THRESHOLD = 18 # 接近阈值
    
    # WARN 级别 (19-23)
    WARN_MINOR = 19       # 轻微异常
    WARN_RETRY = 20       # 重试事件
    WARN_RESOURCE = 21    # 资源接近上限
    WARN_DEPRECATED = 22  # 使用不推荐接口
    WARN_SECURITY = 23    # 可疑安全事件
    
    # ERROR 级别 (24-28)
    ERROR_MINIMAL = 24    # 业务错误
    ERROR_DB = 25         # 数据库错误
    ERROR_INTEGRITY = 26  # 数据一致性问题
    ERROR_DEPENDENCY = 27 # 外部依赖失败
    ERROR_SECURITY = 28   # 已确认安全问题
    
    # CRITICAL 级别 (29-31)
    CRITICAL_SERVICE = 29    # 服务功能不可用
    CRITICAL_PERSIST = 30    # 数据持久化失败
    CRITICAL_DEGRADED = 31   # 系统降级
    
    # FATAL 级别 (32-33)
    FATAL_NODE = 32          # 节点宕机/崩溃
    FATAL_CASCADE = 33       # 级联故障
    
    # EMERGENCY 级别 (34)
    EMERGENCY_SECURITY = 34  # 严重安全事件


class ColorFormatter(logging.Formatter):
    """带颜色的日志格式化器"""
    
    # ANSI 颜色代码
    COLORS = {
        'TRACE': '\033[90m',      # 灰色
        'DEBUG': '\033[36m',      # 青色
        'INFO': '\033[32m',       # 绿色
        'NOTICE': '\033[94m',     # 蓝色
        'WARNING': '\033[33m',    # 黄色
        'ERROR': '\033[31m',      # 红色
        'CRITICAL': '\033[35m',   # 紫色
        'FATAL': '\033[91m',      # 亮红色
        'EMERGENCY': '\033[97;41m', # 白底红字
        'RESET': '\033[0m'
    }
    
    def formatTime(self, record, datefmt=None):
        """自定义时间格式化，支持毫秒"""
        ct = self.converter(record.created)
        if datefmt:
            # 标准时间格式化（不使用 %f）
            s = time.strftime(datefmt, ct)
            # 手动添加毫秒
            msecs = int((record.created - int(record.created)) * 1000)
            s = f"{s}.{msecs:03d}"
        else:
            # 默认格式
            s = time.strftime("%Y-%m-%d %H:%M:%S", ct)
            msecs = int((record.created - int(record.created)) * 1000)
            s = f"{s}.{msecs:03d}"
        return s
    
    def format(self, record):
        # 获取日志级别颜色
        level_name = record.levelname
        color = self.COLORS.get(level_name.split('_')[0], self.COLORS['RESET'])
        
        # 格式化消息
        log_message = super().format(record)
        
        # 添加颜色
        return f"{color}{log_message}{self.COLORS['RESET']}"


class SikuwaLogger:
    """Sikuwa 超详细日志器"""
    
    def __init__(self, name: str, log_dir: Optional[Path] = None, level: int = LogLevel.TRACE_FLOW):
        self.name = name
        self.logger = logging.getLogger(name)
        self.logger.setLevel(1)  # 设置为最低级别，让所有消息都能通过
        self.logger.propagate = False
        
        # 创建日志目录
        if log_dir is None:
            log_dir = Path.cwd() / "sikuwa_logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        
        # 控制台处理器（彩色输出）
        console_handler = logging.StreamHandler(sys.stdout)
        console_handler.setLevel(level)
        console_formatter = ColorFormatter(
            '%(asctime)s [%(levelname)-18s] %(name)s:%(funcName)s:%(lineno)d - %(message)s',
            datefmt='%H:%M:%S'  # 移除 %f，在 formatTime 中手动添加毫秒
        )
        console_handler.setFormatter(console_formatter)
        self.logger.addHandler(console_handler)
        
        # 文件处理器（完整日志）
        timestamp = datetime.now().strftime('%Y%m%d-%H%M%S')
        file_handler = logging.FileHandler(
            log_dir / f"sikuwa-detailed-{timestamp}.log",
            encoding='utf-8'
        )
        file_handler.setLevel(1)
        file_formatter = ColorFormatter(
            '%(asctime)s [%(levelname)-18s] %(name)s:%(funcName)s:%(lineno)d - %(message)s',
            datefmt='%Y-%m-%d %H:%M:%S'  # 移除 %f，在 formatTime 中手动添加毫秒
        )
        file_handler.setFormatter(file_formatter)
        self.logger.addHandler(file_handler)
        
        # 注册自定义级别
        self._register_custom_levels()
    
    def _register_custom_levels(self):
        """注册所有自定义日志级别"""
        for level in LogLevel:
            level_name = level.name
            if not hasattr(logging, level_name):
                logging.addLevelName(level.value, level_name)
    
    # === TRACE 级别快捷方法 ===
    def trace_io(self, msg: str, *args, **kwargs):
        """极细粒度 I/O 跟踪"""
        self.logger.log(LogLevel.TRACE_IO, msg, *args, **kwargs)
    
    def trace_state(self, msg: str, *args, **kwargs):
        """极细粒度状态变更"""
        self.logger.log(LogLevel.TRACE_STATE, msg, *args, **kwargs)
    
    def trace_perf(self, msg: str, *args, **kwargs):
        """微观性能计时"""
        self.logger.log(LogLevel.TRACE_PERF, msg, *args, **kwargs)
    
    def trace_flow(self, msg: str, *args, **kwargs):
        """函数进入退出跟踪"""
        self.logger.log(LogLevel.TRACE_FLOW, msg, *args, **kwargs)
    
    def trace_msg(self, msg: str, *args, **kwargs):
        """消息队列/事件传递"""
        self.logger.log(LogLevel.TRACE_MSG, msg, *args, **kwargs)
    
    # === DEBUG 级别快捷方法 ===
    def debug_detail(self, msg: str, *args, **kwargs):
        """详细调试信息"""
        self.logger.log(LogLevel.DEBUG_DETAIL, msg, *args, **kwargs)
    
    def debug_config(self, msg: str, *args, **kwargs):
        """配置/启动参数"""
        self.logger.log(LogLevel.DEBUG_CONFIG, msg, *args, **kwargs)
    
    def debug_conn(self, msg: str, *args, **kwargs):
        """连接建立/断开"""
        self.logger.log(LogLevel.DEBUG_CONN, msg, *args, **kwargs)
    
    def debug_cache(self, msg: str, *args, **kwargs):
        """缓存命中/失效"""
        self.logger.log(LogLevel.DEBUG_CACHE, msg, *args, **kwargs)
    
    def debug_sql(self, msg: str, *args, **kwargs):
        """SQL/查询执行"""
        self.logger.log(LogLevel.DEBUG_SQL, msg, *args, **kwargs)
    
    # === INFO 级别快捷方法 ===
    def info_operation(self, msg: str, *args, **kwargs):
        """业务操作记录"""
        self.logger.log(LogLevel.INFO_OPERATION, msg, *args, **kwargs)
    
    def info_user(self, msg: str, *args, **kwargs):
        """用户可见操作"""
        self.logger.log(LogLevel.INFO_USER, msg, *args, **kwargs)
    
    def info_metric(self, msg: str, *args, **kwargs):
        """周期性指标快照"""
        self.logger.log(LogLevel.INFO_METRIC, msg, *args, **kwargs)
    
    def info_deploy(self, msg: str, *args, **kwargs):
        """部署/升级事件"""
        self.logger.log(LogLevel.INFO_DEPLOY, msg, *args, **kwargs)
    
    def info_health(self, msg: str, *args, **kwargs):
        """健康检查通过"""
        self.logger.log(LogLevel.INFO_HEALTH, msg, *args, **kwargs)
    
    # === NOTICE 级别快捷方法 ===
    def notice_config(self, msg: str, *args, **kwargs):
        """非致命配置变更"""
        self.logger.log(LogLevel.NOTICE_CONFIG, msg, *args, **kwargs)
    
    def notice_policy(self, msg: str, *args, **kwargs):
        """策略/权限变更"""
        self.logger.log(LogLevel.NOTICE_POLICY, msg, *args, **kwargs)
    
    def notice_threshold(self, msg: str, *args, **kwargs):
        """接近阈值"""
        self.logger.log(LogLevel.NOTICE_THRESHOLD, msg, *args, **kwargs)
    
    # === WARN 级别快捷方法 ===
    def warn_minor(self, msg: str, *args, **kwargs):
        """轻微异常"""
        self.logger.log(LogLevel.WARN_MINOR, msg, *args, **kwargs)
    
    def warn_retry(self, msg: str, *args, **kwargs):
        """重试事件"""
        self.logger.log(LogLevel.WARN_RETRY, msg, *args, **kwargs)
    
    def warn_resource(self, msg: str, *args, **kwargs):
        """资源接近上限"""
        self.logger.log(LogLevel.WARN_RESOURCE, msg, *args, **kwargs)
    
    def warn_deprecated(self, msg: str, *args, **kwargs):
        """使用不推荐接口"""
        self.logger.log(LogLevel.WARN_DEPRECATED, msg, *args, **kwargs)
    
    def warn_security(self, msg: str, *args, **kwargs):
        """可疑安全事件"""
        self.logger.log(LogLevel.WARN_SECURITY, msg, *args, **kwargs)
    
    # === ERROR 级别快捷方法 ===
    def error_minimal(self, msg: str, *args, **kwargs):
        """业务错误"""
        self.logger.log(LogLevel.ERROR_MINIMAL, msg, *args, **kwargs)
    
    def error_db(self, msg: str, *args, **kwargs):
        """数据库错误"""
        self.logger.log(LogLevel.ERROR_DB, msg, *args, **kwargs)
    
    def error_integrity(self, msg: str, *args, **kwargs):
        """数据一致性问题"""
        self.logger.log(LogLevel.ERROR_INTEGRITY, msg, *args, **kwargs)
    
    def error_dependency(self, msg: str, *args, **kwargs):
        """外部依赖失败"""
        self.logger.log(LogLevel.ERROR_DEPENDENCY, msg, *args, **kwargs)
    
    def error_security(self, msg: str, *args, **kwargs):
        """已确认安全问题"""
        self.logger.log(LogLevel.ERROR_SECURITY, msg, *args, **kwargs)
    
    # === CRITICAL 级别快捷方法 ===
    def critical_service(self, msg: str, *args, **kwargs):
        """服务功能不可用"""
        self.logger.log(LogLevel.CRITICAL_SERVICE, msg, *args, **kwargs)
    
    def critical_persist(self, msg: str, *args, **kwargs):
        """数据持久化失败"""
        self.logger.log(LogLevel.CRITICAL_PERSIST, msg, *args, **kwargs)
    
    def critical_degraded(self, msg: str, *args, **kwargs):
        """系统降级"""
        self.logger.log(LogLevel.CRITICAL_DEGRADED, msg, *args, **kwargs)
    
    # === FATAL 级别快捷方法 ===
    def fatal_node(self, msg: str, *args, **kwargs):
        """节点宕机/崩溃"""
        self.logger.log(LogLevel.FATAL_NODE, msg, *args, **kwargs)
    
    def fatal_cascade(self, msg: str, *args, **kwargs):
        """级联故障"""
        self.logger.log(LogLevel.FATAL_CASCADE, msg, *args, **kwargs)
    
    # === EMERGENCY 级别快捷方法 ===
    def emergency_security(self, msg: str, *args, **kwargs):
        """严重安全事件"""
        self.logger.log(LogLevel.EMERGENCY_SECURITY, msg, *args, **kwargs)
    
    # === 装饰器：自动追踪函数执行 ===
    def trace_function(self, func: Callable) -> Callable:
        """装饰器：追踪函数执行"""
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            func_name = func.__name__
            self.trace_flow(f">>> 进入函数: {func_name}")
            self.trace_flow(f"    参数: args={args}, kwargs={kwargs}")
            
            start_time = time.perf_counter()
            try:
                result = func(*args, **kwargs)
                elapsed = (time.perf_counter() - start_time) * 1000
                self.trace_perf(f"    函数 {func_name} 耗时: {elapsed:.3f}ms")
                self.trace_flow(f"<<< 退出函数: {func_name}, 返回值: {result}")
                return result
            except Exception as e:
                elapsed = (time.perf_counter() - start_time) * 1000
                self.error_minimal(f"!!! 函数 {func_name} 异常 (耗时 {elapsed:.3f}ms): {e}")
                raise
        
        return wrapper
    
    def trace_method(self, func: Callable) -> Callable:
        """装饰器：追踪类方法执行"""
        @functools.wraps(func)
        def wrapper(self_obj, *args, **kwargs):
            class_name = self_obj.__class__.__name__
            func_name = func.__name__
            self.trace_flow(f">>> 进入方法: {class_name}.{func_name}")
            self.trace_flow(f"    参数: args={args}, kwargs={kwargs}")
            
            start_time = time.perf_counter()
            try:
                result = func(self_obj, *args, **kwargs)
                elapsed = (time.perf_counter() - start_time) * 1000
                self.trace_perf(f"    方法 {class_name}.{func_name} 耗时: {elapsed:.3f}ms")
                self.trace_flow(f"<<< 退出方法: {class_name}.{func_name}, 返回值类型: {type(result).__name__}")
                return result
            except Exception as e:
                elapsed = (time.perf_counter() - start_time) * 1000
                self.error_minimal(f"!!! 方法 {class_name}.{func_name} 异常 (耗时 {elapsed:.3f}ms): {e}")
                raise
        
        return wrapper


# === 全局日志器实例 ===
_global_logger: Optional[SikuwaLogger] = None


def get_logger(name: str = "sikuwa", level: int = LogLevel.TRACE_FLOW) -> SikuwaLogger:
    """获取全局日志器实例"""
    global _global_logger
    if _global_logger is None:
        _global_logger = SikuwaLogger(name, level=level)
    return _global_logger


def set_log_level(level: int):
    """设置日志级别"""
    logger = get_logger()
    for handler in logger.logger.handlers:
        if isinstance(handler, logging.StreamHandler) and handler.stream == sys.stdout:
            handler.setLevel(level)


# === 便捷函数 ===
def trace_io(msg: str, *args, **kwargs):
    """极细粒度 I/O 跟踪"""
    get_logger().trace_io(msg, *args, **kwargs)


def trace_state(msg: str, *args, **kwargs):
    """极细粒度状态变更"""
    get_logger().trace_state(msg, *args, **kwargs)


def trace_perf(msg: str, *args, **kwargs):
    """微观性能计时"""
    get_logger().trace_perf(msg, *args, **kwargs)


def trace_flow(msg: str, *args, **kwargs):
    """函数进入退出跟踪"""
    get_logger().trace_flow(msg, *args, **kwargs)


def debug_detail(msg: str, *args, **kwargs):
    """详细调试信息"""
    get_logger().debug_detail(msg, *args, **kwargs)


def debug_config(msg: str, *args, **kwargs):
    """配置/启动参数"""
    get_logger().debug_config(msg, *args, **kwargs)


def info_operation(msg: str, *args, **kwargs):
    """业务操作记录"""
    get_logger().info_operation(msg, *args, **kwargs)


def warn_minor(msg: str, *args, **kwargs):
    """轻微异常"""
    get_logger().warn_minor(msg, *args, **kwargs)


def error_minimal(msg: str, *args, **kwargs):
    """业务错误"""
    get_logger().error_minimal(msg, *args, **kwargs)


def critical_service(msg: str, *args, **kwargs):
    """服务功能不可用"""
    get_logger().critical_service(msg, *args, **kwargs)


# === 上下文管理器：性能计时 ===
class PerfTimer:
    """性能计时上下文管理器"""
    
    def __init__(self, name: str, logger: Optional[SikuwaLogger] = None):
        self.name = name
        self.logger = logger or get_logger()
        self.start_time = None
        self.end_time = None
    
    def __enter__(self):
        self.logger.trace_perf(f"  开始计时: {self.name}")
        self.start_time = time.perf_counter()
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.end_time = time.perf_counter()
        elapsed = (self.end_time - self.start_time) * 1000
        
        if exc_type is None:
            self.logger.trace_perf(f" 完成计时: {self.name}, 耗时 {elapsed:.3f}ms")
        else:
            self.logger.trace_perf(f" 异常计时: {self.name}, 耗时 {elapsed:.3f}ms, 异常: {exc_val}")
        
        return False  # 不抑制异常


# === 使用示例 ===
if __name__ == '__main__':
    # 创建日志器
    logger = get_logger("test", level=LogLevel.TRACE_IO)
    
    # 测试所有级别
    logger.trace_io(_("这是 TRACE_IO 级别日志"))
    logger.trace_state(_("这是 TRACE_STATE 级别日志"))
    logger.trace_perf(_("这是 TRACE_PERF 级别日志"))
    logger.trace_flow(_("这是 TRACE_FLOW 级别日志"))
    logger.debug_detail(_("这是 DEBUG_DETAIL 级别日志"))
    logger.debug_config(_("这是 DEBUG_CONFIG 级别日志"))
    logger.info_operation(_("这是 INFO_OPERATION 级别日志"))
    logger.warn_minor(_("这是 WARN_MINOR 级别日志"))
    logger.error_minimal(_("这是 ERROR_MINIMAL 级别日志"))
    logger.critical_service(_("这是 CRITICAL_SERVICE 级别日志"))
    
    # 测试装饰器
    @logger.trace_function
    def test_function(x, y):
        time.sleep(0.1)
        return x + y
    
    result = test_function(1, 2)
    
    # 测试性能计时
    with PerfTimer(_("测试计时块"), logger):
        time.sleep(0.05)
        print(_("执行中..."))
    
    print(f"\n{_('所有测试完成!')}")
