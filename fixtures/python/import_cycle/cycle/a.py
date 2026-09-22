"""Two modules importing each other with unannotated globals that depend on each other."""

from cycle import b

VALUE = b.OTHER + 1
PAIR = (VALUE, b.OTHER)


def read() -> object:
    return b.helper(VALUE)
