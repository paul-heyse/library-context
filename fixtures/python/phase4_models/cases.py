"""Static source data: never executed or linted."""
from typing import cast

def literal():
    return cast(int, 10)

def identity(value):
    return cast(int, value)

def unknown(value):
    return value.unsupported()

from contextlib import nullcontext, suppress


def normal_context():
    with nullcontext():
        pass


def suppressed_context():
    with suppress(TypeError):
        raise None


def preserving_context():
    with nullcontext():
        raise None


def nonmatching_context():
    with suppress(ValueError):
        raise None


def ordered_context():
    with nullcontext(), suppress(TypeError):
        raise None


def finalizing_context():
    with suppress(TypeError):
        try:
            raise None
        finally:
            pass


def valued_context(value):
    with nullcontext(value):
        pass

def targeted_context(value):
    with nullcontext(value) as resource:
        return resource
