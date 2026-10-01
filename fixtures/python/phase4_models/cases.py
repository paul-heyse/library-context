"""Static source data: never executed or linted."""
from typing import cast

def literal():
    return cast(int, 10)

def identity(value):
    return cast(int, value)

def unknown(value):
    return value.unsupported()
