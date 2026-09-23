import logging

logger = logging.getLogger(__name__)


class Server:
    def tool(self, fn):
        return fn

    def run(self):
        return None


def make_server(name: str) -> Server:
    return Server()
