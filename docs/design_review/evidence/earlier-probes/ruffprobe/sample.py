from typing import TYPE_CHECKING

class C:
    def __init__(self, timeout=None, strict=False):
        self.timeout = timeout if timeout is not None else 5
        if strict:
            self.mode = "strict"
        else:
            self.mode = "lax"

    def run(self, x):
        if self.mode == "strict" and x is None:
            raise ValueError("x")
        try:
            y = f(x)
        except KeyError:
            y = None
        finally:
            z = 1
        with open(x) as fh:
            data = fh.read()
        return g(y, self.timeout, self.mode, data, z)

if TYPE_CHECKING:
    T = int
else:
    T = str
print(T)

import sys
if sys.version_info >= (3, 99):
    V = 1
else:
    V = 2
print(V)
