"""Behavior shapes (ADR-0022; the behavioral-model plan's Stage 2.7; increment 3's deep review)."""

from bpkg.deep import top
from bpkg.dispatch import Base, Derived
from bpkg.config import Settings, settings
from bpkg.open import forward_open, own_open
from bpkg.records import Configured, Plain, Point
from bpkg.service import (
    Session,
    configured,
    debug_enabled,
    ignore,
    labelled,
    make,
    open_session,
    paged,
    serve,
    start,
)

__all__ = [
    "Base",
    "Configured",
    "Derived",
    "Plain",
    "Point",
    "Session",
    "Settings",
    "configured",
    "debug_enabled",
    "forward_open",
    "ignore",
    "labelled",
    "make",
    "open_session",
    "own_open",
    "paged",
    "serve",
    "settings",
    "start",
    "top",
]
