class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y


def make_adder(n):
    def add(x):
        return x + n
    return add
