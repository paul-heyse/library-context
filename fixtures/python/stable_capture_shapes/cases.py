def captured_entry(value):
    def inner():
        return value
    return inner()


def captured_literal():
    value = 7
    def inner():
        return value
    return inner()


def mutation(value):
    def inner():
        return value
    value = 9
    return inner()


def call_before_assignment():
    def inner():
        return value
    answer = inner()
    value = 9
    return answer


def escaped(value):
    def inner():
        return value
    return inner


def delayed(value):
    async def inner():
        return value
    return inner()


def nonlocal_write(value):
    def inner():
        nonlocal value
        value = 9
        return value
    return inner()


GLOBAL = 7

def global_read():
    def inner():
        return GLOBAL
    return inner()


def loop_capture(values):
    for value in values:
        def inner():
            return value
        return inner()


def nested_scope(value):
    def middle():
        def inner():
            return value
        return inner()
    return middle()
