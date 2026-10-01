def identity(value):
    return value


def relay(value):
    return identity(value)


def nested(value):
    return identity(identity(value))


def recurse(value):
    if value is None:
        return value
    return recurse(value)


def opaque(value, flag):
    if flag():
        return value
    return opaque(value, flag)


def aliases(value):
    held = identity(value)
    return held


def unresolved(value, function):
    return function(value)


def no_base(value):
    return no_base(value)


def opaque_two(value, left, right):
    if left():
        return value
    if right():
        return opaque_two(value, left, right)
    return opaque_two(value, left, right)


def rebound_alias(value):
    held = identity(value)
    held = None
    return held


# Source association is independent of any claim about a later receiver's heap state.
from dataclasses import dataclass


@dataclass(init=False)
class RecordHolder:
    value: object

    def __init__(self, value):
        self.value = value

    def read(self, flag, other):
        return self.value if flag else ([self.value] if other else unresolved(self.value, other))


class PlainHolder:
    def __init__(self, value):
        self.value = value

    def read(self):
        return self.value
