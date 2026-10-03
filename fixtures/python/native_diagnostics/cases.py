import os
import pytest as ptest

unicode_label = "é λ"
missing_name
suppressed_name  # noqa: F821


def wrong_return() -> int:
    unused_local = "retained"
    return "not an int"


def suppressed_return() -> int:
    return "not an int"  # pyrefly: ignore[bad-return]


@ptest.fixture
def resource() -> int:
    return 7


def test_resource(resource):
    assert resource == 7


def ordinary(resource: int) -> int:
    return resource


@ptest.fixture
def ambiguous() -> int:
    return 1


@ptest.fixture
def ambiguous() -> str:
    return "alternate"


def test_ambiguous(ambiguous):
    assert ambiguous is not None


def test_absent(no_matching_fixture):
    assert no_matching_fixture is not None
