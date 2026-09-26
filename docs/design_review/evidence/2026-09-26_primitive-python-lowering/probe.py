"""CPython 3.14 control for the finite primitive atoms admitted by W11.

Run with ``uv run --no-sync python docs/design_review/evidence/2026-09-26_primitive-python-lowering/probe.py``.
This executes only literal predicates; it never imports or executes the pilot library.
"""

from __future__ import annotations

import json


def evaluate(kind: str, value: object) -> bool:
    if kind == "str_member":
        return value in ("sse", "http")
    if kind == "int_equal":
        return value == 2
    if kind == "str_equal":
        return value == "http"
    if kind == "truthy":
        return bool(value)
    if kind == "is_none":
        return value is None
    if kind == "type_is_str":
        return type(value) is str
    if kind == "int_member":
        return value in (1,)
    if kind == "equal_one":
        return value == 1
    raise AssertionError(kind)


CASES = (
    ("str_member", "http", True),
    ("str_member", "stdio", True),
    ("int_equal", 2, True),
    ("int_equal", 3, True),
    ("str_equal", "http", True),
    ("str_equal", "HTTP", True),
    ("truthy", None, True),
    ("truthy", False, True),
    ("truthy", -1, True),
    ("truthy", "", True),
    ("truthy", "x", True),
    ("is_none", None, True),
    ("is_none", 0, True),
    ("type_is_str", "x", True),
    ("type_is_str", 1, True),
    ("int_member", True, False),
    ("equal_one", True, False),
)


if __name__ == "__main__":
    for kind, value, supported in CASES:
        print(json.dumps({"kind": kind, "value": value, "python": evaluate(kind, value),
                          "lowered": supported}, sort_keys=True))
