"""Isolated int-specialized CrossHair comparison for typing.assert_type."""

from typing import assert_type


def actual(value: int) -> int:
    return assert_type(value, int)


def modeled(value: int) -> int:
    return value


def wrong(value: int) -> int:
    return value + 1
