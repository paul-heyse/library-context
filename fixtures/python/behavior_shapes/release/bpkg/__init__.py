"""Behavior shapes (ADR-0022; the behavioral-model plan's Stage 2.7; increment 3's deep review)."""

from bpkg.deep import top
from bpkg.dispatch import Base, Derived
from bpkg.open import forward_open, own_open
from bpkg.records import Configured, Plain, Point

__all__ = [
    "Base",
    "Configured",
    "Derived",
    "Plain",
    "Point",
    "forward_open",
    "own_open",
    "top",
]
