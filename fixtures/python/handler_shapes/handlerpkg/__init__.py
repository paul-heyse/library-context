"""Source shapes for exact, shadowed and bare except type resolution."""


def pinned_handler(fn):
    try:
        return fn()
    except KeyError:
        return None


def shadowed_handler(fn, KeyError):
    try:
        return fn()
    except KeyError:
        return None


def bare_handler(fn):
    try:
        return fn()
    except:
        return None
