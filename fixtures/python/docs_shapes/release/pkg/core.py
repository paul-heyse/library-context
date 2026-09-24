import logging

logger = logging.getLogger(__name__)


class Server:
    def tool(self, fn):
        return fn

    def run(self):
        """Runs."""

    def stop(self, timeout=None, drain=False, mode=None, retries=0):
        """Stop the server.

        Args:
            drain: Finish the calls in flight first.
        """
        return timeout


def make_server(name: str) -> Server:
    return Server()
