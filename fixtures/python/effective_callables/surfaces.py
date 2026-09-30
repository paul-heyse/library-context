from typing import Callable, ParamSpec, overload
from contextlib import contextmanager
from functools import wraps

def plain(first: int, /, optional: str = "é", *, required: bool, flag: int = 4, **rest: object) -> str:
    return optional

async def asynchronous(value: int) -> int:
    return value

def generator():
    yield 1

def wrapper(function):
    return function

@wrapper
def decorated(value: int) -> int:
    return value

@wraps(plain)
def wrapped(*args, **kwargs):
    return plain(*args, **kwargs)

@contextmanager
def managed():
    yield 1

class Box:
    def ordinary(self, value: int) -> int:
        return value

    @staticmethod
    def static(value: int) -> int:
        return value

    @classmethod
    def create(cls, value: int) -> int:
        return value

    @property
    def value(self) -> int:
        return 1

    @wrapper
    @staticmethod
    def stacked(value: int) -> int:
        return value

def shadowed():
    staticmethod = wrapper
    @staticmethod
    def shadow(value: int) -> int:
        return value
    return shadow

@overload
def variant(value: int) -> int: ...
@overload
def variant(value: str) -> str: ...
def variant(value):
    return value

P = ParamSpec("P")
def forwarded(function: Callable[P, int], *args: P.args, **kwargs: P.kwargs) -> int:
    return function(*args, **kwargs)
