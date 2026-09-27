"""Invocation inputs and reached prefixes; source input, never executed."""
from gzip import compress, decompress
from atexit import register
from typing import cast


def before_raise(value):
    compress(value, compresslevel=9, mtime=0)
    raise None


def after_raise(value):
    raise None
    compress(value, compresslevel=9, mtime=0)


def after_opaque(value, unknown):
    unknown()
    compress(value, compresslevel=9, mtime=0)


def raising_argument(value):
    compress(value, compresslevel=1 / 0, mtime=0)


def missing_defaults(value):
    compress(value)


def selected(value):
    if True:
        compress(value, compresslevel=9, mtime=0)


def skipped(value):
    if False:
        compress(value, compresslevel=9, mtime=0)


def assigned(value):
    result = decompress(value)
    return result


def returned(value):
    return decompress(value)


def register_only(callback):
    register(callback)


def after_total(value):
    cast(object, 1)
    decompress(value)


def after_fallible(value):
    decompress(value)
    compress(value, compresslevel=9, mtime=0)


def nested_argument(value):
    cast(object, decompress(value))


def captured(value):
    def inner():
        decompress(value)
    return inner


async def deferred(value):
    decompress(value)


def generator(value):
    decompress(value)
    yield value


def oversized_arguments(callback):
    register(callback, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
