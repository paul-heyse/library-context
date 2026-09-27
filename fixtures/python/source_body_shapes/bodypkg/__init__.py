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
