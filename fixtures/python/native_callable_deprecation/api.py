from typing import Any, Callable, overload
from warnings import deprecated

@deprecated("  use current\n")
def retired(value: int) -> int:
    return value

def current(value: int) -> int:
    return value

@deprecated("use the text overload")
@overload
def convert(value: int) -> str: ...
@overload
def convert(value: str) -> int: ...
def convert(value: int | str) -> str | int:
    return str(value) if isinstance(value, int) else len(value)

@deprecated("")
def empty_message() -> None:
    pass

def dynamic_message() -> str:
    return "not statically extracted"

@deprecated(dynamic_message())
def unavailable_message() -> None:
    pass

def opaque(fn: Callable[..., Any]) -> Any:
    return fn

@opaque
@deprecated("under opaque wrapper")
def opaque_retired(value: int) -> int:
    return value

class Service:
    @deprecated("use the new method")
    def old(self, value: int) -> str:
        return str(value)

@deprecated("generic deprecated")
def generic[T](value: T) -> T:
    return value

converted = convert(1)
untouched = convert("text")
bound = Service().old


def raises_value_error() -> None:
    raise ValueError("fixture")
