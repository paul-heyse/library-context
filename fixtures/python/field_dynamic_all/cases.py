"""Unconstrained name-driven access input; never executed."""


class Cold:
    def __init__(self, value):
        self.unread = value


def dynamic(code, value, ignored):
    exec(code)
    eval(code)
    vars(value)
    return value
