"""Type shapes (CPG slice C4): structure, binders, record fields and raised types."""

import dataclasses
import enum
import sys
from collections.abc import Callable
from typing import (
    Generic,
    Literal,
    NamedTuple,
    NotRequired,
    Optional,
    ParamSpec,
    Protocol,
    TypedDict,
    TypeVar,
    dataclass_transform,
)

T = TypeVar("T")
P = ParamSpec("P")


# Two unrelated `T`s: one term each, told apart by their binders.
def first(xs: list[T]) -> T:
    return xs[0]


def ident(x: T) -> T:
    return x


def maybe(x: Optional[int], y: int | str | None = None) -> dict[str, list[int]]:
    return {}


def wrap(f: Callable[P, T]) -> Callable[P, list[T]]:
    def inner(*args: P.args, **kwargs: P.kwargs) -> list[T]:
        return [f(*args, **kwargs)]

    return inner


# A class-scoped `T`, bound by the class.
class Box(Generic[T]):
    def get(self) -> T: ...

    def put(self, item: T) -> None: ...


# PEP 695 parameters.
class Pair[K, V]:
    def swap(self, k: K, v: V) -> tuple[V, K]: ...


# A recursive alias: a reference to its name, never an expansion.
type Tree = int | list[Tree]


def leaves(t: Tree) -> int:
    return 0


class Greeter(Protocol):
    def greet(self, name: str) -> str: ...


class Movie(TypedDict):
    title: str
    year: NotRequired[int]


class Point(NamedTuple):
    x: int
    y: int = 0


@dataclasses.dataclass
class Config:
    name: str
    retries: int = 3
    tags: list[str] = dataclasses.field(default_factory=list)
    token: str = dataclasses.field(default="", kw_only=True)
    internal: int = dataclasses.field(default=0, init=False)


@dataclass_transform()
def model[C](cls: type[C]) -> type[C]:
    return cls


@model
class Settings:
    host: str
    port: int = 8080


class ConfigError(ValueError):
    pass


def load(path: str) -> Config:
    if not path:
        raise ConfigError("empty path")
    if path == "?":
        raise ValueError
    return Config(name=path)


def use(g: Greeter) -> None:
    load("x")
    first([1, 2])
    Box[int]().put(3)
    Pair[str, int]().swap("a", 1)
    leaves(3)
    g.greet("you")
    Point(1)
    Settings(host="h")


# C4 review shapes.
async def fetch(n: int) -> str:  # declared `str`, not the coroutine Pyrefly computes
    return str(n)


B = TypeVar("B", bound=int)


def clamp(x: B) -> B:  # `B`'s bound is a child of its term
    return x


class Color(enum.Enum):
    RED = 1


def paint(c: Literal[Color.RED]) -> None: ...


@dataclasses.dataclass
class BaseRec:
    x: int = 0


@dataclasses.dataclass
class SubRec(BaseRec):
    def __post_init__(self) -> None:
        self.x = 1  # an inherited field assigned in a method is still the base's


def later(items: list[int]) -> list[int]:
    return sorted(items, key=lambda i: abs(i))  # a call inside a lambda body


if sys.platform == "win32":
    windows_only = print("never analyzed on linux")
