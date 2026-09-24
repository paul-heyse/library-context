"""A chain reaching the depth bound (2) with a read of the formal at the frontier."""


def top(value):
    return middle(value)


def middle(value):
    return bottom(value)


def bottom(value):
    return leaf(value + 1)


def leaf(n):
    return n
