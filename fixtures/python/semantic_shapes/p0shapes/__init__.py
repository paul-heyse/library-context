"""P0 known-answer shapes for one transfer relation (design review 2026-09-29).

Expected: timeout reaches Config.timeout and read_timeout's return; title never reaches timeout;
the two identity calls in `pair` do not cross (first never reaches b); `fetch` passes timeout and
fallback to `transport` under complementary guards but nothing reaches fetch's return through the
unresolved `transport`; the facade forwards x unchanged; `open_call` stays open.
"""


class Config:
    def __init__(self, timeout: float = 30.0, title: str = "x") -> None:
        self.timeout = timeout
        self.title = title


def read_timeout(config: Config) -> float:
    return config.timeout


def identity(value):
    return value


def pair(first, second):
    a = identity(first)
    b = identity(second)
    return b


def select_timeout(timeout, fallback):
    if timeout is None:
        return fallback
    return timeout


def fetch(url, timeout=None, fallback=30):
    selected = select_timeout(timeout, fallback)
    return transport(url, timeout=selected)  # noqa: F821 - unresolved on purpose


def facade(x):
    return _inner(x)


def _inner(y):
    return y


class Service:
    def run(self, x):
        return self._inner(x)

    def _inner(self, x):
        return x


def open_call(v):
    return unknown_function(v)  # noqa: F821 - unresolved on purpose
