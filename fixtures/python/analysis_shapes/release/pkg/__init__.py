"""Pass A's known-answer shapes (DESIGN §9.1, §12): the seed is `pkg.Server.tool`."""

from pkg.alpha import Alpha
from pkg.controls import configure
from pkg.helpers import describe, helper
from pkg.registry import Catalog, Widgets
from pkg.server import Server
from pkg.shadow import Aliased, Shadowed

__all__ = [
    "Aliased",
    "Alpha",
    "Catalog",
    "Server",
    "Shadowed",
    "Widgets",
    "configure",
    "describe",
    "helper",
]
