# Café: a non-ASCII byte before the docstrings, so evidence offsets are bytes (review F8).

def finish(fn):
    return fn


def helper(fn, n=1):
    """Run `fn` through the finishing step — once."""
    return finish(fn)


def prepare(fn):
    return finish(fn)


def deep_three():
    return 3


def deep_two():
    return deep_three()


def deep_one():
    return deep_two()
