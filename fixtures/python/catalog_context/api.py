"""C1 original scenario fixture; captured input only."""
from dataclasses import dataclass

__all__ = ["connect", "Config", "PlainConfig"]

def connect(host: str, *, timeout: int = 2):
    return host

@dataclass
class Config:
    left: str = "http"
    right: int = 16

    def read_left(self):
        if self.left:
            return self.left
        return self.left

    def read_right(self):
        if self.right:
            return self.right
        return self.right

def example(host):
    previous = "before"
    with unavailable_context():
        connect(host, timeout=3)
    return previous

class PlainConfig:
    left: str
    right: int

    def __init__(self, left: str, right: int = 16):
        self.left = left
        self.right = right

    def read_left(self):
        if self.left:
            return self.left
        return self.left

def construct_config(left: str):
    return PlainConfig(left, 17)

def reinitialize_config(config: PlainConfig, left: str):
    PlainConfig.__init__(config, left, 17)
