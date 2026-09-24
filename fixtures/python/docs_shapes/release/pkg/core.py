import logging

logger = logging.getLogger(__name__)


class Server:
    def tool(self, fn):
        return fn

    def run(self):
        """Runs."""

    def stop(self, timeout=None):
        """Stop the server."""
        return timeout


def make_server(name: str) -> Server:
    return Server()
