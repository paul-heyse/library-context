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


def no_base(value):
    return no_base(value)


def opaque_two(value, left, right):
    if left():
        return value
    if right():
        return opaque_two(value, left, right)
    return opaque_two(value, left, right)
