"""A referenced stdlib callable for the Stage 3 authored-model binding fixture."""

import atexit
import gzip
import json
import logging
from pydantic.type_adapter import TypeAdapter
from typing import Callable, assert_type, cast


def identity(value: object) -> object:
    return cast(object, value)


def identity_literal_type(value: object) -> object:
    return cast("object", value)


def identity_dynamic_type(value: object, typ: type[object]) -> object:
    return cast(typ, value)


def identity_shadowed_type(value: object, object: object) -> object:
    return cast(object, value)


def identity_raising_sibling(value: object) -> object:
    return cast(1 / 0, value)


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


def indirect_ambiguous(value: object, choose: bool) -> object:
    if choose:
        result = cast(object, value)
    else:
        result = value
    return result


def nested_identity(value: object) -> object:
    return cast(object, str(value))


def nested_total_identity(value: object) -> object:
    return cast(object, cast(object, value))


def nested_three_total_identity(value: object) -> object:
    return cast(object, cast(object, cast(object, value)))


def nested_raising_identity(value: object) -> object:
    return cast(object, cast(1 / 0, value))


def nested_deleted_identity(value: object) -> object:
    del value
    return cast(object, cast(object, value))


def computed_identity(value: object) -> object:
    return cast(object, value) or "fallback"


def asserted_type(value: object) -> object:
    return assert_type(value, object)


def json_text(value: object) -> str:
    return json.dumps(value)


def json_write(value: object, stream) -> None:
    json.dump(value, stream)


def json_decode(value: str) -> object:
    return json.loads(value)


def compress_data(value: bytes) -> bytes:
    return gzip.compress(value)


def decompress_data(value: bytes) -> bytes:
    return gzip.decompress(value)


def shadowed_compress(gzip, value: bytes):
    return gzip.compress(value)


def warn(value: str) -> None:
    logging.getLogger(__name__).warning(value)


def shadowed_warn(logger, value: str) -> None:
    logger.warning(value)


def warn_unpacked(values: tuple[str, ...]) -> None:
    logging.getLogger(__name__).warning(*values)


def plain_identity(value: object) -> object:
    return value


def local_wrapper(value: object) -> object:
    return plain_identity(value)


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


def framed_modeled_identity(value: object) -> object:
    try:
        return cast(object, value)
    finally:
        pass


def nested_framed_modeled_identity(value: object) -> object:
    try:
        try:
            return cast(object, value)
        finally:
            pass
    finally:
        pass


def validate_data(value: object) -> int:
    return TypeAdapter(int).validate_python(value)


def shadowed_adapter(adapter, value: object):
    return adapter.validate_python(value)
