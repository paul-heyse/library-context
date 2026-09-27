"""Static source data: never executed or linted."""


def fallthrough():
    pass


def explicit_return():
    """Return a pair of closed literal values."""
    return (1, "closed")


def first_initialization():
    unused = (1, 2)
    return 3


def selected_literal():
    if True:
        return 4
    else:
        unknown()


def preserved_return():
    try:
        return 5
    finally:
        pass


def replaced_return():
    try:
        return 6
    finally:
        return 7


def replaced_raise():
    try:
        return 8
    finally:
        raise None


def explicit_raise():
    raise None


def unknown_owned_value():
    unused = object()
    return 9


def initialized_local_read():
    value = 10
    return value


def default_parameter(value=11):
    return 12


def required_parameter(value):
    return 13


def closure_owner(value):
    def captured():
        if False:
            return value
        return 14
    return captured()


def dynamic_definition():
    def nested():
        return 15
    return 16


async def deferred_async():
    return 17


def deferred_generator():
    yield 18


@unknown_decorator
def decorated():
    return 19


def reassignment():
    value = 20
    value = 21
    return 22


def proof_at_limit():
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    return 23


def proof_over_limit():
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    return 24


def docstring_only():
    """Only definition-time metadata."""


def call_literal(value):
    """Keep a value after a fresh literal-returning call completes."""
    def inner():
        return (1, "closed")
    inner()
    return value


def call_fallthrough(value):
    def inner():
        """Definition metadata only."""
    inner()
    return value


def call_return_finally(value):
    def inner():
        try:
            return 2
        finally:
            return 3
    inner()
    return value


def call_raises(value):
    def inner():
        raise None
    inner()
    return value


def call_local_read(value):
    def inner():
        local = 4
        return local
    inner()
    return value


def call_default(value):
    def inner(unused=5):
        return 6
    inner()
    return value


def call_captured(value):
    def inner():
        if False:
            return value
        return 7
    inner()
    return value


def call_intervening(value):
    def inner():
        return 8
    pass
    inner()
    return value


def call_alias(value):
    def inner():
        return 9
    alias = inner
    alias()
    return value


from typing import cast


def call_modeled(value):
    def inner():
        return cast(int, 10)
    inner()
    return value


def call_unreachable(value):
    raise None
    def inner():
        return 11
    inner()
    return value


from json import dump


def call_prefix_small(value):
    def inner():
        return 12
    inner()
    pass
    dump(value, value)
    return value


def call_prefix_over_limit(value):
    def inner():
        return 12
    inner()
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    pass
    dump(value, value)
    return value


def call_failed_header(value):
    @value
    def inner():
        return 13
    inner()
    return value


def call_unknown_body(value):
    def inner():
        unresolved()
    inner()
    return value


def call_generator(value):
    def inner():
        yield 14
    inner()
    return value


def call_unreachable_yield(value):
    def inner():
        if False:
            yield 15
        return 16
    inner()
    return value
