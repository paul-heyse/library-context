"""Independent CPython controls; never imported by extraction or used as compiler input."""

import json
import sys

assert sys.version_info[:3] == (3, 14, 7), sys.version
cases = [
    ("(not False and True and True) if True else 1 / 0", True),
    ("(not False and True and True) if False else 1 / 0", ZeroDivisionError),
    ("False and (1 / 0) and True", False),
    ("True or (1 / 0) or False", True),
    ("not False and True and (1 / 0)", ZeroDivisionError),
    ("False or False or (1 / 0)", ZeroDivisionError),
    ("not False and True and 42", 42),
    ("(-0x10 + (2.5 - -3)) if not False else 1 / 0", -10.5),
    ("10**400 + 1.0", OverflowError),
]
rows = []
for source, expected in cases:
    try:
        actual = eval(source, {"__builtins__": {}})
    except (ZeroDivisionError, OverflowError) as error:
        assert type(error) is expected, (source, error)
        rows.append({"expression": source, "outcome": type(error).__name__})
    else:
        assert type(actual) is type(expected) and actual == expected, (source, actual)
        rows.append({"expression": source, "outcome": "normal", "type": type(actual).__name__, "value": actual})
print(json.dumps({"python": sys.version, "cases": rows}, indent=2))
