import sys
from dataclasses import dataclass
from typing import TYPE_CHECKING

from depmod import helper


def lonely():
    """Public, and nothing calls it: an isolate of the call graph."""
    return 1


def callee(x):
    return x


def twice(x):
    # Two call sites, one callee: two parallel edges.
    callee(x)
    return callee(x)


def fact(n):
    # Direct recursion: a self-loop.
    return 1 if n <= 1 else n * fact(n - 1)


def left(x):
    return bottom(x)


def right(x):
    return bottom(x)


def bottom(x):
    # Into the dependency.
    return helper(x)


def top(x):
    # A diamond: top -> left, right -> bottom.
    return left(x) + right(x)


def dispatch(obj, name):
    # The outer call's target is computed: unresolved.
    return getattr(obj, name)()


def unresolved(obj, name):
    return dispatch(obj, name)


@dataclass
class Point:
    x: int
    y: int


def make_point():
    # The dataclass's synthesized __init__.
    return Point(1, 2)


def double(x):
    return 2 * x


def apply_all(xs):
    # A higher-order argument.
    return list(map(double, xs))


def nested(x):
    # f(g(x)): the argument and the inner call are distinct nodes.
    return callee(double(x))


if TYPE_CHECKING:

    def mode() -> str: ...

else:

    def mode():
        return "runtime"


if sys.version_info >= (3, 12):

    def compat(x):
        return x

else:

    def compat(x):
        return None
