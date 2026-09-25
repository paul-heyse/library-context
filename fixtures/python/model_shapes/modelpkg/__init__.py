"""A referenced stdlib callable for the Stage 3 authored-model binding fixture."""

from typing import cast


def identity(value: object) -> object:
    return cast(object, value)
