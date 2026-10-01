"""Static flow and original official usage controls; never executed by the compiler."""
__all__ = ["produce", "consume", "spare", "forward", "alias_forward", "computed", "rebound", "guarded", "caught", "unpacked", "swallowed", "tested", "literal"]

def produce() -> int:
    return 7

def consume(value: int) -> int:
    return value

def spare(value: int) -> int:
    return value

def forward(value: int) -> int:
    return consume(value)

def alias_forward(value: int) -> int:
    alias = value
    return consume(alias)

def computed(value: int) -> int:
    return consume(value + 1)

def rebound(value: int) -> int:
    value = 2
    return consume(value)

def guarded(value: int) -> int:
    if value < 0:
        raise ValueError("negative")
    return value

def caught(value: int) -> int:
    try:
        return guarded(value)
    except ValueError:
        return 0

def unpacked(value: tuple[int, ...]) -> int:
    return consume(*value)


def swallowed(value: int) -> int:
    try:
        if value < 0:
            raise ValueError("negative")
    except ValueError:
        return 0
    return value

def tested(value: int) -> int:
    if value > 0:
        return guarded(value)
    return value


def literal() -> int:
    return consume(4)
