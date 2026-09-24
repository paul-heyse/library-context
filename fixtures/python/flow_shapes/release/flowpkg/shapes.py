"""One function per shape; each test in `cpg-flow` names its function and line."""

import os
import sys
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

    marker = "checker"
else:
    marker = "runtime"

if sys.version_info >= (3, 12):
    version_branch = "new"
else:
    version_branch = "old"

settings = object()


def fallback(timeout=None):
    timeout = timeout if timeout is not None else 30
    return timeout


def settings_fallback(host=None):
    if host is None:
        host = settings.host
    return host


def guarded(path, strict=False):
    if not path:
        raise ValueError("empty")
    if strict:
        return path.upper()
    return path


def either(a, b):
    return a or b


def walrus(items):
    if (n := len(items)) > 3:
        return n
    return 0


def comprehension(prefix, names):
    return [prefix + n for n in names]


def loop(values):
    total = 0
    for v in values:
        if v is None:
            continue
        if v == "stop":
            break
        total = total + v
    else:
        total = -1
    return total


def handled(fn):
    result = None
    try:
        result = fn()
    except KeyError as error:
        result = error
    finally:
        cleanup = True
    return result, cleanup


def suppressed(cm, compute):
    value = 1
    with cm:
        value = compute()
    return value


def matched(command):
    match command:
        case "start":
            state = 1
        case None:
            state = 0
        case _:
            state = -1
    return state


counter = 0


def bump():
    global counter
    counter = counter + 1
    return counter


def outer():
    level = 1

    def inner():
        nonlocal level
        level = level + 1
        return level

    return inner


def deleted(data):
    copy = data
    del copy
    return data


def modes(transport, host=None):
    if transport in ("http", "sse"):
        return host
    return None


def type_checking_flag(value):
    if TYPE_CHECKING:
        value = "checker"
    return value


def augmented(n):
    n += 1
    return n


def platform_branch():
    if os.name == "nt":
        return "windows"
    return "posix"


def empty_loop():
    last = None
    for item in []:
        last = item
    return last


def elif_chain(mode):
    if mode == "a":
        result = 1
    elif mode == "b":
        result = 2
    else:
        result = 3
    return result


def safe_try():
    marker_value = 0
    try:
        marker_value = 1
    except KeyboardInterrupt:
        marker_value = 2
    return marker_value


if TYPE_CHECKING:

    def both(x: int) -> int: ...

else:

    def both(x):
        return x


TEXT_WITH_WORD = "TYPE_CHECKING"  # TYPE_CHECKING in a comment
F_TEXT = f"{TYPE_CHECKING}"


def string_mode(flag):
    if flag == "TYPE_CHECKING":  # a comment
        return 1
    return 0


def commented(a, b):
    if (a  # first
            > b):
        return a
    return b


def nested_fallback(level=None, name=None):
    chosen = (level if level is not None else settings.level).lower()
    label = f"[{name}]"
    return fallback(name), chosen, label


def alias_fallback(stateless=None, stateless_http=None):
    if stateless is not None and stateless_http is None:
        stateless_http = stateless
    if stateless_http is None:
        stateless_http = settings.stateless
    return stateless_http
