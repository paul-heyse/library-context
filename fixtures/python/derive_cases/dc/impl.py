import functools
from dataclasses import dataclass


@dataclass
class Point:
    x: int
    y: int


def _helper(v):
    return v


class Holder:
    fn = staticmethod(_helper)


def make(x):
    return x


def wrapped(x):
    return x


wrapped = functools.lru_cache(wrapped)
