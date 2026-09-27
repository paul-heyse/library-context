from contextlib import nullcontext, suppress


def preserve(value):
    with nullcontext():
        return value


def entry_value(value):
    with nullcontext(value) as chosen:
        return chosen


def suppress_type_error(value):
    with suppress(TypeError):
        raise None
    return value


def multiple(value):
    with nullcontext(), suppress(TypeError):
        raise None
    return value


def unsupported(value, manager):
    with manager:
        return value


async def deferred(value):
    async with nullcontext():
        return value


def nonmatching(value):
    with suppress(ValueError):
        raise None
    return value


def assignment_failure_suppressed(value):
    with suppress(TypeError), nullcontext() as (left, right):
        return -1
    return value


def constructor_failure_suppressed(value):
    with suppress(TypeError), nullcontext(1, 2):
        return -1
    return value


def replacement_suppressed(value):
    with suppress(TypeError), suppress(1):
        raise None
    return value


def replacement_preserved(value):
    with nullcontext(), suppress(1):
        raise None
    return value


def return_preserved(value):
    with suppress(1):
        return value


def matching_short_circuit(value):
    with suppress(TypeError, 1):
        raise None
    return value


def invalid_first(value):
    with suppress(1, TypeError):
        raise None
    return value


def break_preserved(value):
    for item in (1,):
        with suppress(1):
            break
    return value


def continue_preserved(value):
    for item in (1,):
        with suppress(1):
            continue
    return value


def unknown_body(value, effect):
    with nullcontext():
        effect()
    return value


def generator(value):
    with nullcontext():
        yield value
