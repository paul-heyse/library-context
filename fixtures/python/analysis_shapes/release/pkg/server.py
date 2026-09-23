from dataclasses import dataclass

import extdep

from pkg.handlers import Handler, run
from pkg.helpers import deep_one, helper, prepare
from pkg.other import outside


@dataclass
class Config:
    size: int


class Server:
    def __init__(self) -> None:
        self.name = "server"

    @property
    def label(self) -> str:
        # A property getter that delegates.
        return prepare(self.name)

    def dispatch(self) -> int:
        return deep_one()

    def route(self, path):
        """Register a handler at `path`."""

        # A decorator factory: its work is in the nested callable it returns.
        def decorator(fn):
            helper(fn)
            return fn

        return decorator

    def tool(self, fn):
        """Register `fn` as a tool."""
        helper(fn)
        helper(fn, 2)
        prepare(fn)
        self.dispatch()
        run(Handler())
        extdep.external(fn)
        outside()
        Config(1)
        getattr(fn, "x")()
        callback = helper
        _ = self.label
        return fn, callback
