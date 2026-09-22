"""Every Pysa call-graph variant the mapper must place (DESIGN §4.2.3)."""

from typing import Annotated, Callable


class Marker:
    def __init__(self, label: str) -> None:
        self.label = label


def register(func: Callable[..., int]) -> Callable[..., int]:
    return func


class Base:
    def run(self, x: int) -> int:
        return x

    @property
    def size(self) -> int:
        return 1

    @size.setter
    def size(self, value: int) -> None:
        self._size = value


class Child(Base):
    def __new__(cls, *args: object) -> "Child":
        return super().__new__(cls)

    def __init__(self, start: int = 0) -> None:
        self.start = start

    def run(self, x: int) -> int:
        return super().run(x) + self.start


def dispatch(obj: Base, value: int) -> int:
    return obj.run(value)


def apply(fn: Callable[[int], int], value: int) -> int:
    return fn(value)


def double(v: int) -> int:
    return v * 2


@register
def decorated(v: int) -> int:
    def inner(w: int) -> int:
        return w + v

    return inner(v)


def uses(label: Annotated[str, Marker("metadata")] = "x") -> str:
    child = Child(3)
    child.size = 4
    width = child.size
    result = apply(double, dispatch(child, width))
    name = getattr(child, "start")
    lazy = child.run
    return f"{label}:{result!r}:{name}:{lazy}" if result > 0 else str(result)


MODULE_LEVEL = double(21)


class Plain:
    size: int = 0


def union_property(x: Base | Plain) -> int:
    return x.size


import typing  # appended here so the byte offsets above stay put


def name_partly_unknown(flag: bool, other: typing.Any) -> object:
    h = double if flag else other
    return h


def artificial_getattr(child: Child) -> int:
    return getattr(child, "size", 0)
