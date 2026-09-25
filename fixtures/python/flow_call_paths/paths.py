"""Small source shapes for ordered call-result provenance; analyzed, never executed."""


def direct(value):
    return cast(object, value)


def nested(value):
    return outer(inner(data=value))


def callee(func, value):
    return func(value)


def computed(value):
    return outer(value + 1)
