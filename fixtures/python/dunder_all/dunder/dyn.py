def _names() -> list[str]:
    return ["dyn_public", "dyn_other"]


__all__ = _names()


def dyn_public() -> int:
    return 3


def dyn_other() -> int:
    return 4


def alpha_call() -> int:
    return 5
