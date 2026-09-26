"""Finite summary cap and independent normal-completion control."""


def f0(value: object) -> object:
    """Return the supplied value unchanged."""
    return value


def f1(value: object) -> object:
    return f0(value)


def f2(value: object) -> object:
    return f1(value)


def f3(value: object) -> object:
    return f2(value)


def f4(value: object) -> object:
    return f3(value)


def f5(value: object) -> object:
    return f4(value)


def f6(value: object) -> object:
    return f5(value)


def f7(value: object) -> object:
    return f6(value)


def f8(value: object) -> object:
    return f7(value)


def f9(value: object) -> object:
    """Return the supplied value after nine local identity calls."""
    return f8(value)


def opaque() -> None:
    pass


def unsupported(value: object) -> object:
    """Return the value after an opaque preceding call."""
    opaque()
    return value
