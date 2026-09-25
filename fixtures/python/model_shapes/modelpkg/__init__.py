"""A referenced stdlib callable for the Stage 3 authored-model binding fixture."""

import atexit
from typing import Callable, cast


def identity(value: object) -> object:
    return cast(object, value)


def display(value: object) -> None:
    print(value)


def acquire(path: str):
    return open(path)


def on_shutdown(callback: Callable[[], None]) -> Callable[[], None]:
    return atexit.register(callback)


def shadowed_open(open):
    return open("local argument, not the builtin")
