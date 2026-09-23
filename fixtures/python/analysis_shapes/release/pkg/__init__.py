"""Pass A's known-answer shapes (DESIGN §9.1, §12): the seed is `pkg.Server.tool`."""

from pkg.helpers import helper
from pkg.server import Server
from pkg.shadow import Aliased, Shadowed

__all__ = ["Aliased", "Server", "Shadowed", "helper"]
