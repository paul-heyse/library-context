"""Lexical shapes (CPG slice C3): each resolution rule Python's scoping has."""

import os.path
from typing import TYPE_CHECKING

from lex.helpers import *  # noqa: F403

if TYPE_CHECKING:
    from collections.abc import Sequence
else:
    Sequence = list

counter = 0
limit = 10
limit = 20  # shadows the first binding


def bump():
    global counter
    counter += 1
    return counter


def outer(scale):
    total = 0

    def inner(x):
        nonlocal total
        total += x * scale  # `scale` is captured
        return total

    return inner


class Config:
    name = "config"
    names = [name for _ in range(2)]  # the first iterable sees the class; the element does not

    def describe(self):
        return name_or_default(self)  # a method cannot see the class's `name`


def name_or_default(obj):
    return getattr(obj, "name", None) or "default"


def filtered(items):
    doubled = [x * 2 for x in items if (y := x) > 0]
    return doubled, y, (lambda z: z + limit)(1)


def shadowed_builtin(values):
    len = 3  # noqa: A001
    return len, max(values), os.path.join("a", "b")


def uses_star():
    return helper_from_star()


def dropped():
    tmp = 1
    del tmp
