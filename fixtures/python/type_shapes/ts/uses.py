"""Types observed from another module (C4 review F3, F5)."""

import enum
from typing import Literal

from ts import ident


class Color(enum.Enum):  # same name as `ts.Color`, another class
    RED = 1


def paint_here(c: Literal[Color.RED]) -> None: ...


def use_ident() -> int:
    print(ident)  # the generic observed outside its module: one term per variable
    f = ident
    return f(3)
