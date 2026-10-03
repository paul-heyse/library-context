from typing import overload, Callable
__all__ = ['parse', 'replacement', 'Box', 'apply']
@overload
def parse(value: int) -> int: ...
@overload
def parse(value: str) -> str: ...
def parse(value: int | str) -> int | str:
    return value
def replacement(value: int) -> int:
    return value + 1
class Box:
    def run(self, value: int) -> int:
        return value

def apply(callback: Callable[[int], int], value: int) -> int:
    return callback(value)
