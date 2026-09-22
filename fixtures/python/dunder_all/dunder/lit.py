__all__ = ["exported"]
__all__ += ["also"]
__all__.append("third")


def exported() -> int:
    return 1


def also() -> int:
    return 2


def third() -> int:
    return 3


def hidden() -> int:
    return 4
