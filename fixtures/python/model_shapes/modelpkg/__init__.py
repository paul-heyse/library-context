"""A referenced stdlib callable for the Stage 3 authored-model binding fixture."""

import atexit
import json
from typing import Callable, assert_type, cast


def identity(value: object) -> object:
    return cast(object, value)


def identity_literal_type(value: object) -> object:
    return cast("object", value)


def identity_dynamic_type(value: object, typ: type[object]) -> object:
    return cast(typ, value)


def identity_shadowed_type(value: object, object: object) -> object:
    return cast(object, value)


def display(value: object) -> None:
    print(value)


def acquire(path: str):
    return open(path)


def on_shutdown(callback: Callable[[], None]) -> Callable[[], None]:
    return atexit.register(callback)


def shadowed_open(open):
    return open("local argument, not the builtin")


def on_shutdown_keyword(callback: Callable[[], None]) -> Callable[[], None]:
    return atexit.register(func=callback)


def on_shutdown_unpacked(callbacks):
    return atexit.register(*callbacks)


def identity_keyword(value: object) -> object:
    return cast(typ=object, val=value)


def indirect_identity(value: object) -> object:
    result = cast(object, value)
    return result


def nested_identity(value: object) -> object:
    return cast(object, str(value))


def computed_identity(value: object) -> object:
    return cast(object, value) or "fallback"


def asserted_type(value: object) -> object:
    return assert_type(value, object)


def json_text(value: object) -> str:
    return json.dumps(value)


def json_write(value: object, stream) -> None:
    json.dump(value, stream)


def plain_identity(value: object) -> object:
    return value


async def async_identity(value: object) -> object:
    return value


def generator_identity(value: object):
    yield value
    return value


def framed_identity(value: object) -> object:
    try:
        return value
    finally:
        pass
