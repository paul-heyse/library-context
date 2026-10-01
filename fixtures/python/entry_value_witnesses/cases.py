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
