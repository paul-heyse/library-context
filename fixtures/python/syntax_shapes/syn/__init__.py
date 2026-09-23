"""Syntax shapes (CPG slice C2): the constructs Passes B and C read."""

import functools


class ConfigError(ValueError):
    pass


def guarded(x, mode="fast", limit=0):
    # A guard: a predicate on a parameter, then a local raise.
    if x is None:
        raise ValueError("x is required")
    elif mode not in ("fast", "safe"):
        raise ConfigError(f"unknown mode {mode}") from None
    if 0 <= limit < 10:
        limit += 1
    assert limit >= 0, "limit is negative"
    return x


def handled(path):
    try:
        data = open(path).read()
    except (OSError, ValueError) as err:
        raise ConfigError("cannot read") from err
    else:
        return data
    finally:
        pass


def matched(command):
    match command:
        case {"op": op} if (n := len(op)) > 3:
            return n
        case _:
            return 0


def looped(items):
    total = 0
    for item in items:
        with open(item) as fh:
            total += len(fh.read())
    return total


@functools.lru_cache(maxsize=8)
def decorated(key):
    return key.upper()
