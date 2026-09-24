"""One function per shape; each test in `cpg-flow` names its function and line."""

import os
import sys
from typing import TYPE_CHECKING
from typing import TYPE_CHECKING as TC
import typing as typing_alias

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


def two_calls(probe):
    if probe():
        if not probe():
            return "second call changed"
    return "other"


def two_ranges(n, m):
    found = -1
    for i in range(n):
        found = i
    for j in range(m):
        found = j
    return found


def mutated(self):
    if self.x is None:
        self.reset()
        if self.x is not None:
            return "mutated"
    return "other"


def cleared(items):
    if items:
        items.clear()
        if not items:
            return "cleared"
    return "other"


def eq_vs_is(x):
    if x == True:
        if x is not True:
            return "equal, not identical"
    return "other"


def eq_none(x):
    if x == None:
        if x is not None:
            return "equal, not identical"
    return "other"


def same_line_bindings(x, y):
    if x is None:
        x = y; x = None
    if x is None:
        return x
    return y


def merged_definitions(x, choose):
    if choose:
        x = None
    if x is None:
        return "merge"
    return "other"


def choose(TYPE_CHECKING):
    if TYPE_CHECKING:
        return "parameter"
    return "other"


def checking_alias():
    if TC:
        return "checker only"
    return "runtime"


def checking_module_alias():
    if typing_alias.TYPE_CHECKING:
        return "checker only"
    return "runtime"


def config_check(config):
    if config.TYPE_CHECKING:
        return "ordinary attribute"
    return "other"


def version_prefix():
    if sys.version_info > (3, 14):
        above = True
    else:
        above = False
    if sys.version_info >= (3, 14):
        at_least = True
    else:
        at_least = False
    if sys.version_info <= (3, 14):
        at_most = True
    else:
        at_most = False
    if sys.version_info == (3, 14):
        equal = True
    else:
        equal = False
    return above, at_least, at_most, equal


def version_micro_prefix():
    if sys.version_info > (3, 14, 7):
        above_micro = True
    else:
        above_micro = False
    if sys.version_info <= (3, 14, 7):
        at_most_micro = True
    else:
        at_most_micro = False
    if sys.version_info == (3, 14, 7):
        equal_micro = True
    else:
        equal_micro = False
    return above_micro, at_most_micro, equal_micro


def rebound_isinstance_class(x):
    C = int
    if isinstance(x, C):
        C = str
        if not isinstance(x, C):
            return "rebound class"
    return "other"


def non_singleton_identity(x):
    if x is 1000:
        if x is not 1000:
            return "different literal object"
    return "other"


def nonlocal_change():
    x = None

    def change():
        nonlocal x
        x = 1

    if x is None:
        change()
        if x is not None:
            return "changed nonlocal"
    return "other"


def value_runtime_branch(x, y):
    return x if TYPE_CHECKING else y
