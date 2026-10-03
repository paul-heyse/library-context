from typing import TYPE_CHECKING as TC
from collections import deque as Queue

sentinel = 1

def local(value: int) -> int:
    rebound = value
    rebound = rebound + 1
    return rebound

def outer():
    captured = 3
    def nested():
        return captured
    return nested()

class Box:
    def method(self) -> int:
        return missing

try:
    raise ValueError()
except ValueError as error:
    observed = error

del observed
values = [item for item in range(3)]

def generic[T](value: T) -> T:
    return value

if TC:
    typed_only = Queue[int]
