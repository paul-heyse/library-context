class Box:
    def __init__(self, v: int):
        self.v = v

    def __eq__(self, other: object) -> bool:
        return other is None or (isinstance(other, Box) and other.v == self.v)


def eq_none(x: Box) -> bool:
    return x == None  # noqa: E711


def is_none(x: Box) -> bool:
    return x is None


def eq_true(x: int) -> bool:
    return x == True  # noqa: E712


def is_true(x: int) -> bool:
    return x is True
