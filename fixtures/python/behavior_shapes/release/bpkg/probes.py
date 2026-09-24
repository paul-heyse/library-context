"""The Stage 2 end review's probes (R1, R2, R4-R8, R10): each shape with the answer it must get."""

import abc
import contextlib
from typing import TYPE_CHECKING


def sink(value):
    return value


def h(value):
    return value


def g(value):
    return value


def accumulate(items, p):
    cur = None
    for _ in items:
        if cur is None:
            cur = p
        else:
            sink(cur)


def opaque_rebind(name, default="x"):
    if "." in name:
        name = default
    if "." in name:
        raise ValueError("dotted")
    sink(name)


def typed_rebind(name=None, default="x"):
    if name is None:
        name = default
    if name is None:
        raise ValueError("missing")
    sink(name)


def cycle(p):
    x = p
    while h(x):
        y = h(x)
        x = g(y)


def caught(x, y):
    try:
        if x is None:
            raise LookupError("missing")
        sink(y)
    except LookupError:
        pass


def suppressed(x, y):
    with contextlib.suppress(KeyError):
        if x is None:
            raise KeyError("missing")
        sink(y)


def overwrite(mode, other):
    mode = other
    if mode == "http":
        return sink(1)
    return None


class Holder:
    def __init__(self, token):
        self._token = token


def make_holder(token):
    return Holder(token)


def reader():
    return make_holder("t")._token


class Handler:
    def on_message(self, message):
        """A hook: a subclass reads the message."""

    def render(self, arguments):
        raise NotImplementedError

    def concrete(self, unused):
        return 1


class LoudHandler(Handler):
    def on_message(self, message):
        return sink(message)


class Job(abc.ABC):
    @abc.abstractmethod
    def run(self, work): ...


class Config:
    if TYPE_CHECKING:

        def __init__(self, data): ...


class Snapshot:
    def __init__(self, a):
        self.a = a

    def dump(self):
        return dict(self.__dict__)
