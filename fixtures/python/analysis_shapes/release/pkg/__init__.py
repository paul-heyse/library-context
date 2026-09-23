"""Pass A's known-answer shapes (DESIGN §9.1, §12): the seed is `pkg.Server.tool`."""

from pkg.helpers import helper
from pkg.server import Server

__all__ = ["Server", "helper"]
