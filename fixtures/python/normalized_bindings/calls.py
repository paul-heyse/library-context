from typing import Callable, TypeVar, overload

T = TypeVar("T")


def plain(x: int, y: int = 2) -> int:
    return x + y


def wrapper(fn: Callable[[int], int]) -> Callable[[int], int]:
    return fn


@wrapper
def decorated(x: int) -> int:
    return x


@overload
def variant(x: int) -> int: ...
@overload
def variant(x: int, y: int) -> int: ...
def variant(x: int, y: int = 0) -> int:
    return x + y


def run(items: list[int]) -> None:
    plain(1)
    plain(1, 2, 3)
    plain(*items)
    decorated(1)
    variant(1)
