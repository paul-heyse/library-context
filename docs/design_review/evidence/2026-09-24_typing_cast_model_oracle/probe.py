"""Isolated CrossHair oracle for the committed typing.cast identity model.

This checks the int specialization of the pure return-value claim. The deliberately wrong
control establishes that this invocation can find a difference on the same argument domain.
"""

from typing import cast


def actual(value: int) -> int:
    return cast(int, value)


def modeled(value: int) -> int:
    return value


def wrong(value: int) -> int:
    return value + 1
