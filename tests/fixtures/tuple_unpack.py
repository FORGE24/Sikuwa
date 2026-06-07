def swap(a, b):
    a, b = b, a
    return a + b

def fib_step(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a
