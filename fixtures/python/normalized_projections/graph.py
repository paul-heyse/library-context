from helper import outside


def a():
    b()
    c()
    b()
    outside()


def b():
    d()


def c():
    d()


def d():
    a()


def isolated():
    return 0


def recursive():
    recursive()


def unresolved(callback):
    callback()


class DispatchBase:
    def invoke(self, value):
        return value


class DispatchLeft(DispatchBase):
    def invoke(self, value):
        return dispatch_relay(self, value)


class DispatchRight(DispatchBase):
    def invoke(self, value):
        return value


class DispatchDiamond(DispatchLeft, DispatchRight):
    def invoke(self, value):
        return value


def dispatch_relay(obj: DispatchBase, value):
    return obj.invoke(value)


def factory():
    def nested():
        return 1

    return nested


def shortest_entry():
    shortest_helper()
    shortest_target()


def shortest_helper():
    shortest_target()


def shortest_target():
    return 1


def frontier_entry(callback):
    frontier_child(callback)


def frontier_child(callback):
    callback()


def closed_entry():
    shortest_target()
