"""Input-only controls for ordered expression completion; never executed."""
from typing import cast


def selected_parameter(value, typ):
    return cast(typ if True else missing, value)


def selected_builtin(value):
    return cast(object if True else missing, value)


def selected_missing(value):
    return cast(missing if True else object, value)


def unknown_truthiness(value, typ):
    return cast(not typ, value)


def deleted_read(value, typ):
    del typ
    return cast(typ if True else object, value)


def short_circuited(value, typ):
    return cast(False and typ, value)


def nested_call_sibling(value):
    return cast(cast(object, object), value)


def nested_call_selected(value):
    return cast(cast(object, object) if True else missing, value)


def nested_call_raising(value):
    return cast(cast(1 / 0, object), value)


def missing_required_predecessor(value):
    cast(object)
    return value


def extra_argument_predecessor(value):
    cast(object, value, 1)
    return value


def extra_keyword_result(value):
    return cast(object, value, extra=1)


def nested_call_predecessor(value):
    cast(cast(object, object), value)
    return value
def normal_call_finalizer(value):
    try:
        return value
    finally:
        cast(object, 1)


def raising_call_finalizer(value):
    try:
        return value
    finally:
        cast(object, 1 / 0)


def overriding_finalizer(value):
    try:
        return value
    finally:
        return None


def selected_finalizer(value):
    try:
        return value
    finally:
        if True:
            marker = 1
        else:
            missing()


def unknown_finalizer(value, enabled):
    try:
        return value
    finally:
        if enabled:
            missing()


def opaque_exception_finalizer(value, exception):
    try:
        return value
    finally:
        try:
            raise exception
        except:
            pass


def rebinding_finalizer(value, previous):
    try:
        return value
    finally:
        previous = 1


def caught_primitive_finalizer(value):
    try:
        return value
    finally:
        try:
            raise None
        except:
            pass


def repeated_initialization(value, iterable):
    for item in iterable:
        marker = item
    return value
