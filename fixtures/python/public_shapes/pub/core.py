"""Each public-path rule, one class per case."""

import json
from typing import overload


def helper(x):
    return x


class Base:
    def __init__(self):
        self.ready = True

    def __repr__(self):
        return "Base"

    def run(self):
        """Runs."""

    def stop(self):
        """Stops."""

    def _hidden(self):
        pass


class Child(Base):
    """Inherits `run`, overrides `stop`."""

    def stop(self):
        """Stops the child."""


class Rebinds(Base):
    """Binds `run` by an assignment: no `Rebinds.run`."""

    run = helper


class Unresolved(Missing, Base):  # noqa: F821
    """An undefined base: Pyrefly leaves it out of the MRO, so `Base`'s members are inherited."""


class External(json.JSONDecoder, Base):
    """An ancestor outside the release precedes `Base`: nothing is inherited."""


class Over:
    @overload
    def f(self, x: int) -> int: ...

    @overload
    def f(self, x: str) -> str: ...

    def f(self, x):
        return x


class Outer:
    class Inner:
        def m(self):
            pass


class _Private(Base):
    pass
