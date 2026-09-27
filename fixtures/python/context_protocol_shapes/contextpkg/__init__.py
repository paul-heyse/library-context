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


def entry_keyword(value):
    with nullcontext(enter_result=value) as chosen:
        return chosen


def entry_other(value, other):
    with nullcontext(other) as chosen:
        return chosen


def entry_after_context(value):
    with nullcontext(value) as chosen:
        pass
    return chosen


def entry_rebound(value):
    with nullcontext(value) as chosen:
        chosen = None
        return chosen


def entry_deleted(value):
    with nullcontext(value) as chosen:
        del chosen
        return chosen


def entry_nested_mutation(value):
    with nullcontext(value) as chosen:
        def change():
            nonlocal chosen
            chosen = None
        return chosen


def entry_none(value):
    with nullcontext() as chosen:
        return chosen


def entry_suppress(value):
    with suppress(TypeError) as chosen:
        return chosen


def entry_sibling(value):
    with nullcontext(value) as first:
        pass
    with nullcontext() as chosen:
        return chosen


def entry_overridden(value):
    with nullcontext(value) as chosen:
        try:
            return chosen
        finally:
            return None


def entry_opaque_predecessor(value, effect):
    with nullcontext(value) as chosen:
        effect()
        return chosen
