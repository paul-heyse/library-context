"""Grounded documentary fixture; captured input only."""
__all__ = ["connect", "alias", "undocumented", "escaped"]

def connect(host: str, *, timeout: int = 2):
    """Connect — carefully
    to the authored endpoint. A second sentence.

    Warning:
        Authentication remains the caller's responsibility.
    """
    return host

alias = connect

def undocumented(value):
    return value

def escaped():
    "A mapped\\nsource must remain uncertain."
    return None
