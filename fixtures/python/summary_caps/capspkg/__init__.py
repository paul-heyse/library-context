"""Finite summary cap and independent normal-completion control."""

from typing import cast


def f0(value: object) -> object:
    """Return the supplied value unchanged."""
    return value


def f1(value: object) -> object:
    return f0(value)


def f2(value: object) -> object:
    return f1(value)


def f3(value: object) -> object:
    return f2(value)


def f4(value: object) -> object:
    return f3(value)


def f5(value: object) -> object:
    return f4(value)


def f6(value: object) -> object:
    return f5(value)


def f7(value: object) -> object:
    return f6(value)


def f8(value: object) -> object:
    return f7(value)


def f9(value: object) -> object:
    """Return the supplied value after nine local identity calls."""
    return f8(value)


def opaque() -> None:
    pass


def unsupported(value: object) -> object:
    """Return the value after an opaque preceding call."""
    opaque()
    return value


def completed_predecessor(value: object) -> object:
    """A pinned total call with two simple arguments precedes the direct return."""
    cast(object, 1)
    return value


def parameter_predecessor(value: object) -> object:
    """A bound formal can be evaluated before the normal model call."""
    cast(object, value)
    return value


def possibly_unbound_argument(value: object, other: object, clear: bool) -> object:
    """A parameter read with a possible deletion is not a normal argument witness."""
    if clear:
        del other
    cast(object, other)
    return value


def raising_predecessor(value: object) -> object:
    """An unevaluated sibling must not borrow the callee's normal-return claim."""
    cast(1 / 0, 1)
    return value


def conditional_callee(value: object, enabled: bool) -> object:
    """A flow-insensitive callee binding cannot prove an evaluated call."""
    if enabled:
        from typing import cast as local_cast
    local_cast(object, 1)
    return value


if __name__ == "only_in_one_runtime":
    from typing import cast as guarded_cast


def guarded_module_callee(value: object) -> object:
    """A conditional module import cannot establish callee availability."""
    guarded_cast(object, 1)
    return value
