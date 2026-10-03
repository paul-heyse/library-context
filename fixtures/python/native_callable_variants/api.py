from typing import Any, Callable, Generic, ParamSpec, TypeVar, overload
from dataclasses import dataclass

def wrap(fn: Callable[[int], int]):
    def implementation(text: str, *, flag: bool = False) -> bytes:
        return text.encode()
    return implementation

@wrap
def changed(value: int) -> int:
    return value

def opaque(fn: Callable[..., Any]) -> Any:
    return fn

@opaque
def unavailable(value: int) -> int:
    return value

@overload
def parse(value: int) -> str: ...
@overload
def parse(value: str) -> int: ...
def parse(value: int | str) -> str | int:
    return str(value) if isinstance(value, int) else len(value)

@dataclass
class Settings:
    host: str
    port: int = 80

class Service:
    def run(self, value: int) -> str:
        return str(value)

P = ParamSpec("P")
R = TypeVar("R")
def preserve(fn: Callable[P, R]) -> Callable[P, R]:
    return fn

@preserve
def retained(value: float) -> bool:
    return value > 0

chosen = parse(1)
not_chosen = parse(object())
bound = Service().run
result = bound(1)

class Wrapper[**Q, S]:
    def __init__(self, callback: Callable[Q, S]):
        self.callback = callback

    def __call__(self, *args: Q.args, **kwargs: Q.kwargs) -> S:
        return self.callback(*args, **kwargs)


wrapped_parse = Wrapper(parse)
wrapped_result = wrapped_parse(1)
