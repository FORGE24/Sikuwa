# Plan 8a.2 — after `if x == 1`, x is narrowed to int on the then branch.

def add_one_if_one(x):
    if x == 1:
        return x + 1
    return 0

def after_none_guard(x):
    if x is None:
        return 0
    return x + 1
