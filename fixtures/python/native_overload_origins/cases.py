from typing import overload

@overload
def choose(value: int) -> int: ...
@overload
def choose(value: str) -> str: ...
def choose(value): return value

@overload
def equal_shape(value: int) -> int: ...
@overload
def equal_shape(value: int) -> int: ...
def equal_shape(value): return value

@overload
def recover(value: int) -> int: ...
@overload
def recover(value: str, other: str) -> str: ...
def recover(value, other=None): return value

def identity[T](value: T) -> T: return value

class Reader:
    def read(self, value: int) -> int: return value

def exercise(value: int | str):
    selected = choose(1)
    failed = choose(1.5)
    expanded = choose(value)
    equal = equal_shape(1)
    recovered = recover(1.5)
    generic = identity(1)
    bound = Reader().read(1)
    return selected, failed, expanded, equal, recovered, generic, bound
