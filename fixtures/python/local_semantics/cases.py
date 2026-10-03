def parameter(value):
    if value is None:
        return value
    return value


def assigned(value):
    value = None
    if value is None:
        return value


def unbound(value, remove):
    if remove:
        del value
    if value is None:
        return value


def loop(value, items):
    for item in items:
        value = item
    if value is None:
        return value


def captured(value):
    def nested():
        if value is None:
            return value
    return nested


def nonlocal_write(value):
    def change():
        nonlocal value
        value = None
    change()
    if value is None:
        return value


class Receiver:
    def method(self):
        if self is None:
            return self

    @classmethod
    def class_method(cls):
        if cls is None:
            return cls


def caller(obj: Receiver):
    parameter(None)
    obj.method()
    obj.class_method()


def crossed(value):
    return parameter(value)


def outer_guard(value, gate):
    if gate:
        if value is None:
            return value

from typing import Literal

class Root:
    pass

class Left(Root):
    pass

class Right(Root):
    pass

class Diamond(Left, Right):
    pass

def finite_strings(value: Literal["http", "sse"]):
    if value == "http":
        return value
    if value in ("http", "sse"):
        return value

def finite_ints(value: Literal[1, 2]):
    if value == 3:
        return value
    return value

def finite_none(value: Literal[None]):
    if value is None:
        return value

def finite_bools(value: Literal[True, False]):
    if value:
        return value

def open_hierarchy(value: Diamond):
    if isinstance(value, Root):
        return value

def opaque_domain(value):
    if value == "http":
        return value

def finite_type(value: Literal[1, 2]):
    if type(value) is int:
        return value
    if isinstance(value, str):
        return value
    return None

def shadowed_type(value: Literal[1, 2], type):
    if type(value) is int:
        return value
    return None

def shadowed_instance(value: Literal[1, 2], isinstance):
    if isinstance(value, str):
        return value
    return None

class Fields:
    left: int
    right: str

    def read_left(self):
        if self.left:
            return self.left
        return None

    def read_right(self):
        if self.right:
            return self.right
        return None

    def rebinding(self):
        self = Fields()
        if self.left:
            return self.left
        return None

    def mutation(self):
        self.left = 4
        if self.left:
            return self.left
        return None

class Unrelated:
    left: str

    def read_left(self):
        if self.left:
            return self.left
        return None

def open_nonmember(value: Diamond):
    if isinstance(value, Unrelated):
        return value
    return None

def open_mixed(value: Diamond | Unrelated):
    if isinstance(value, Root):
        return value
    return None


def alias_assignment(value):
    alias = value
    return alias


def rebound_assignment(value):
    value = 1
    alias = value
    return alias


from typing import Never


def finite_zero(value: Literal[0]):
    if value:
        return value
    return value


def finite_one(value: Literal[1]):
    if value:
        return value
    return value


def uninhabited(value: Never):
    if value:
        return value
    return value


def nonconforming_runtime():
    # This call violates the typing premise; it cannot erase the conservative branch.
    return finite_zero(1)


def finite_compound(value: Literal[0], gate: bool):
    if value and gate:
        return value
    return value
