from dataclasses import dataclass
from typing import Callable

@dataclass
class Service:
    value: int
    callback: Callable[[int], str]

    def method(self, value: int = 1) -> int:
        return value


def factory():
    class Nested:
        def method(self):
            return 2
    return Nested


__all__ = ["Service", "factory", "missing"]
