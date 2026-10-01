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


def call_modeled_parameter(value):
    return cast(int, value)


def call_modeled_keyword_order():
    return cast(val=10, typ=int)


def call_literal_argument():
    def inner(value):
        return value
    return inner(17)


def call_held_argument(value):
    def inner(item):
        return item
    return inner(value)


def call_unavailable_argument():
    def inner(item):
        return item
    return inner(missing_actual)


def call_argument_keyword_order(value):
    def inner(first, second):
        return first
    return inner(second=19, first=value)


def literal_default_header(value):
    def inner(enabled=True):
        return enabled
    return value


def keyword_default_header(value):
    def inner(*, enabled=False):
        return enabled
    return value


def unexecuted_body_header(value):
    def inner(enabled=True):
        return missing_body_name()
    return value


def missing_default_header(value):
    def inner(enabled=missing_default_name()):
        return enabled
    return value


def rebound_definition_header(value):
    inner = value
    def inner():
        return 1
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
