from typing import cast

def effect():
    return None

def read(value):
    return value

def expressions():
    a = False and effect()
    b = True or effect()
    c = 3 if True else effect()
    d = 1 + 2
    e = -2
    f = not ()
    g = "hello"
    h = 999999999999999999999999999999999999999 + 1
    i = True and effect()
    return a

def finally_returns():
    try:
        raise None
    finally:
        return 1

def finally_raises():
    try:
        return 1
    finally:
        raise None

def bare_handler():
    try:
        raise None
    except:
        raise

def handler_name_cleanup():
    try:
        raise None
    except TypeError as error:
        return None


def complex_value():
    return 1j


def body_pass():
    pass


def body_stops():
    return 1
    effect()


def field_read(value):
    return value.field


async def async_body():
    pass


def generator_body():
    return 1
    yield 2


def fresh_source():
    def fresh():
        return None
    return fresh()


def builtin_positive():
    return int


def builtin_shadowed(int):
    return int


def builtin_rebound():
    int = 1
    return int


def fresh_modeled_source():
    def fresh():
        return cast(int, 10)

    return fresh()


def fresh_literal_argument():
    def fresh(item):
        return item
    fresh(7)
    return 1


def fresh_held_argument(value):
    def fresh(item):
        return item
    fresh(value)
    return value


def default_header(value):
    def fresh(enabled=True):
        return missing_body_name()
    return value


from contextlib import nullcontext, suppress


def with_preserve():
    with nullcontext(7):
        pass


def with_preserve_return():
    with nullcontext():
        return 31


def with_suppress():
    with suppress(TypeError):
        raise None


def with_nonmatch():
    with suppress(ValueError):
        raise None


def with_multiple():
    with nullcontext(7), suppress(TypeError):
        raise None


def with_nested():
    with nullcontext():
        with suppress(TypeError):
            raise None


def with_unknown_body():
    with nullcontext():
        unresolved_body()


async def with_async():
    async with nullcontext():
        pass


def with_target():
    with nullcontext(37) as value:
        return value


def with_finalizer_return():
    with suppress(TypeError):
        try:
            raise None
        finally:
            return 41


def with_none_target():
    with nullcontext() as value:
        return value


def with_rebound_target():
    value = 5
    with nullcontext(7) as value:
        return value


def with_missing_target_constructor():
    with unknown_context() as value:
        return value


def with_held_target(value):
    with nullcontext(value) as resource:
        return resource
