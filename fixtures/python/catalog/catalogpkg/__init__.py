"""Catalog-only contract fixture; input data, never executed by extraction."""
from dataclasses import dataclass
from typing import overload

def ordinary(first: int, /, optional: str = "ok", *items: int, flag: bool = False, **options: str) -> str:
    """Return a value using the declared controls.

    Args:
        optional: The original optional text.
        flag: A keyword-only switch.
    """
    return optional

alias = ordinary

def undocumented():
    pass

def wrapper(function):
    return function

@wrapper
def wrapped(value: int = None):
    return value

@overload
def choose(value: int, /) -> int: ...
@overload
def choose(value: str, *, prefix: str = "") -> str: ...
def choose(value, **options):
    return value

class Base:
    def method(self, *, count: int = 1):
        return count

class Child(Base):
    pass

@dataclass
class Config:
    count: int
    title: str = "sample"
