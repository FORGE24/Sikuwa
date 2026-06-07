import time
import sys
from functools import lru_cache

# ---------- 不同的斐波那契实现 ----------
def fib_recursive(n: int) -> int:
    """朴素递归（非常慢，仅适合 n <= 35）"""
    if n <= 1:
        return n
    return fib_recursive(n - 1) + fib_recursive(n - 2)

@lru_cache(maxsize=None)
def fib_memo(n: int) -> int:
    """记忆化递归（快，适合中等 n）"""
    if n <= 1:
        return n
    return fib_memo(n - 1) + fib_memo(n - 2)

def fib_iterative(n: int) -> int:
    """迭代法（快速，适合大 n）"""
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a

def fib_matrix(n: int) -> int:
    """矩阵快速幂（极快，适合超大 n）"""
    def mat_mul(a, b):
        return [
            [a[0][0] * b[0][0] + a[0][1] * b[1][0], a[0][0] * b[0][1] + a[0][1] * b[1][1]],
            [a[1][0] * b[0][0] + a[1][1] * b[1][0], a[1][0] * b[0][1] + a[1][1] * b[1][1]]
        ]

    def mat_pow(mat, exp):
        res = [[1, 0], [0, 1]]  # 单位矩阵
        while exp:
            if exp & 1:
                res = mat_mul(res, mat)
            mat = mat_mul(mat, mat)
            exp >>= 1
        return res

    if n == 0:
        return 0
    base = [[1, 1], [1, 0]]
    result = mat_pow(base, n - 1)
    return result[0][0]

# ---------- 测速工具 ----------
def benchmark(func, n: int, repeat: int = 1) -> float:
    """测量 func(n) 的平均耗时（秒），重复 repeat 次取平均"""
    # 预热一次（针对缓存、JIT等），不计时
    try:
        _ = func(n)
    except:
        pass

    total = 0.0
    for _ in range(repeat):
        start = time.perf_counter()
        try:
            _ = func(n)
        except RecursionError:
            return float('inf')  # 递归溢出
        except Exception as e:
            print(f"    {func.__name__} 出错: {e}")
            return float('inf')
        total += time.perf_counter() - start
    return total / repeat

# ---------- 主测试 ----------
def main():
    # 调整递归栈限制，防止递归测试轻易崩溃
    sys.setrecursionlimit(10000)

    test_cases = [
        (30, 1),    # n=30，只测1次（递归勉强能跑）
        (35, 1),    # n=35，递归会很慢，注意等待
        (100, 100), # n=100，跳过递归，其余方法多跑几次取平均
        (1000, 100),
        (10000, 10),
    ]

    functions = {
        "递归(朴素)": fib_recursive,
        "递归(记忆化)": fib_memo,
        "迭代": fib_iterative,
        "矩阵快速幂": fib_matrix,
    }

    print("=" * 70)
    print("斐波那契数列计算速度对比 (时间单位: 秒)")
    print("=" * 70)

    for n, repeat in test_cases:
        print(f"\n>>> n = {n} (重复 {repeat} 次取平均)")
        print("-" * 40)
        for name, func in functions.items():
            # 朴素递归只测 n <= 35，否则太慢/崩溃
            if name == "递归(朴素)" and n > 35:
                print(f"  {name:16s}: 跳过 (n 太大)")
                continue
            t = benchmark(func, n, repeat)
            if t == float('inf'):
                print(f"  {name:16s}: 递归深度溢出")
            else:
                print(f"  {name:16s}: {t:.6f} 秒")

if __name__ == "__main__":
    main()