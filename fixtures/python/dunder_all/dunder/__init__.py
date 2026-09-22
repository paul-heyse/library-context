"""`__all__` built from a submodule's `__all__`: Pyrefly cannot resolve it statically."""

from . import sub
from .sub import *  # noqa: F403
from .dyn import dyn_public

__all__ = sub.__all__ + ["top"]


def top() -> int:
    return dyn_public()
