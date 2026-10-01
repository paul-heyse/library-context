"""C1 original scenario fixture; captured input only."""
from dataclasses import dataclass

__all__ = ["connect", "Config"]

def connect(host: str, *, timeout: int = 2):
    return host

@dataclass
class Config:
    left: str = "http"
    right: int = 16

    def read_left(self):
        return self.left

    def read_right(self):
        return self.right

def example(host):
    previous = "before"
    with unavailable_context():
        connect(host, timeout=3)
    return previous
