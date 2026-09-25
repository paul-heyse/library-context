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


def opaque_with(manager):
    with manager:
        raise ValueError("opaque context exit")


def finally_overrides():
    try:
        raise ValueError("overridden")
    finally:
        return None


def unrelated_handler():
    try:
        raise ValueError("requires identity proof")
    except TypeError:
        return None


def plain_raise():
    raise ValueError("unframed")
