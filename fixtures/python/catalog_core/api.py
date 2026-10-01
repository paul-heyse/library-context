"""Mandatory catalog fixture: input data, never executed."""
from dataclasses import dataclass, field
from typing import overload
from contextlib import contextmanager
from functools import wraps

__all__ = ['choose', 'alias', 'wrapped', 'Base', 'Child', 'Config', 'ChildConfig', 'managed', 'decorated', 'Accessors']

@overload
def choose(value: int, /) -> int: ...
@overload
def choose(value: str, *, prefix: str = '') -> str: ...
def choose(value, **options):
    return value

alias = choose

def replace(function):
    return function

@replace
def wrapped(value=None):
    return value

class Base:
    def __init__(self, *, count: int = 1):
        self.count = count

class Child(Base):
    pass

@dataclass
class Config:
    left: str = 'http'
    right: int = 16
    cache: list[str] = field(default_factory=list)

    def read_left(self):
        return self.left

@dataclass
class ChildConfig(Config):
    extra: bool = False


@contextmanager
def managed():
    yield 1

@wraps(wrapped)
def decorated(value=None):
    return wrapped(value)

class Accessors:
    @property
    def value(self):
        return 1

    @value.setter
    def value(self, value):
        pass

    @value.deleter
    def value(self):
        pass
