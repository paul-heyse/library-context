from contextlib import nullcontext, suppress


def optional_value():
    with nullcontext(7) as value:
        return value


def absent_value():
    with nullcontext() as value:
        return value


def suppressed_error():
    with suppress(TypeError):
        raise TypeError()
