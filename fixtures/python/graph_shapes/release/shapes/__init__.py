"""Graph shapes (ADR-0014, guidelines §12): each public name pins one shape of the catalog."""

from depmod import helper as helper
from shapes.core import (
    apply_all,
    compat,
    dispatch,
    fact,
    lonely,
    make_point,
    mode,
    nested,
    top,
    twice,
    unresolved,
)
from shapes.models import Base, Child, Plain
from shapes.ping import ping
from shapes.typed import overloaded

VERSION = "1.0"

__all__ = [
    "Base",
    "Child",
    "Plain",
    "VERSION",
    "apply_all",
    "compat",
    "dispatch",
    "fact",
    "helper",
    "lonely",
    "make_point",
    "mode",
    "nested",
    "overloaded",
    "ping",
    "top",
    "twice",
    "unresolved",
]
