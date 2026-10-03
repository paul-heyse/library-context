from typing import Callable, ParamSpec, TypeVar, Generic

class Box[T]:
    def __init__(self, value: T): self.value = value
    def get(self) -> T: return self.value

class Other[T]:
    def __init__(self, value: T): self.value = value
    def get(self) -> T: return self.value

integer_box = Box[int](1)
string_box = Box[str]("x")
integer_result = integer_box.get()
string_result = string_box.get()
other_result = Other[bytes](b"x").get()

Cov = TypeVar("Cov", covariant=True)
Choice = TypeVar("Choice", int, str, default=str)
class Legacy(Generic[Cov]):
    def get(self) -> Cov: raise NotImplementedError

def choose(value: Choice) -> Choice: return value

P = ParamSpec("P")
R = TypeVar("R")
def preserve(callback: Callable[P, R]) -> Callable[P, R]: return callback
@preserve
def retained(value: float) -> bool: return True

type Recursive = int | list[Recursive]
def recursive(value: Recursive) -> Recursive: return value

retained_result = retained(1.0)
recursive_result = recursive([1])
