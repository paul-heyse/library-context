import sys


def _new(x):
    return x


def _old(x):
    return x


if sys.version_info >= (3, 11):
    def load(x):
        return _new(x)
else:
    def load(x):
        return _old(x)
