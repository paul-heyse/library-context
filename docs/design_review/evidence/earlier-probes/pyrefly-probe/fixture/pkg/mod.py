import sys
from typing import TYPE_CHECKING
import typing
import os

if TYPE_CHECKING:
    from collections import OrderedDict as D
else:
    D = dict

if not TYPE_CHECKING:
    R = 1

if sys.version_info >= (3, 12):
    V = "new"
elif sys.platform == "win32":
    V = "win"
else:
    V = "old"

if typing.TYPE_CHECKING:
    T2 = int

class Cfg:
    version_info_cache = None

cfg = Cfg()
if cfg.version_info_cache:
    W = 1

if os.name == "nt":
    N = 1

if flag := os.environ.get("X"):
    pass
elif sys.version_info < (3, 9):
    OLD = True


def f(flag):
    """Doc of f."""
    x = 1
    if flag:
        x = 2
    return x


class C:
    '''Doc of C.'''

    def m(self):
        return self.helper()

    def helper(self):
        return f(True)


async def g() -> int:
    return 1


y = C().m()
print(D, V, W, N, R, T2, y, cfg.version_info_cache)
