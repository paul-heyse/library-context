from typing import Generic, TypeVar
import missing_package

T = TypeVar("T")

class Base:
    pass

class Legacy(Generic[T]):
    def identity(self, value: T) -> T:
        return value

class Box[T](Base):
    item: T

    def method(self, value: T) -> T:
        return value

def identity[T](value: T) -> T:
    return value

def _private(value: int) -> int:
    return value

def choose(value: int | str) -> int:
    if isinstance(value, int):
        return value
    return len(value)

__all__ = ["Base", "Box", "identity", "choose"]
