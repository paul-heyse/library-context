from typing import overload

def consume(values: list[int]) -> None:
    pass

assigned: list[int] = []
consume([])

@overload
def overloaded(value: int) -> int: ...
@overload
def overloaded(value: str) -> str: ...
def overloaded(value):
    return value

overloaded(1.5)
