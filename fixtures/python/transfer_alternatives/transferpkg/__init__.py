"""Input data for condition-safe transfer projection; never executed."""


def opaque(value):
    return object()


def sink(value):
    return value


def mixed(flag, other, value):
    return value if flag else ([value] if other else opaque(value))


def forward(flag, other, value):
    return sink(value if flag else ([value] if other else opaque(value)))


def outer(flag, other, value):
    return forward(flag, other, value)


def captured(value):
    return lambda: opaque(value)


def replace(function):
    return lambda *args: None


@replace
def decorated(value):
    return value


@replace
def decorated_empty():
    pass


def through_decorated(value):
    return decorated(value)


class Holder:
    def __init__(self, flag, other, value):
        self.value = value

    def read(self, flag, other):
        return self.value if flag else ([self.value] if other else opaque(self.value))


class ConditionalHolder:
    def __init__(self, flag, value):
        if flag:
            self.saved = value

    def read(self):
        return self.saved


class Stable:
    def __init__(self, value):
        self.saved = value

    def lookup(self, name):
        alias = self
        return getattr(alias, name)

    def dictionary(self):
        alias = self
        return alias.__dict__


class MixedReceiver:
    def lookup(self, flag, other, name):
        alias = self if flag else other
        return getattr(alias, name)

    def dictionary(self, flag):
        alias = self if flag else opaque(self)
        return alias.__dict__

    def loop(self, values, name):
        alias = self
        for value in values:
            alias = object()
        return getattr(alias, name)


class Unrelated:
    def __init__(self, value):
        self.unread = value


def unused(value, ignored):
    return value


class ChoiceA:
    def __init__(self):
        self.a = None


class ChoiceB:
    def __init__(self):
        self.b = None


switch = object()
if switch:
    global_choice = ChoiceA()
else:
    global_choice = ChoiceB()

rebound = ChoiceA()
rebound = ChoiceB()
fixed = ChoiceA()


def read_global_choice(name):
    return getattr(global_choice, name)


def read_rebound(name):
    return getattr(rebound, name)


def read_fixed(name):
    return getattr(fixed, name)


if switch:
    class Selected:
        selected_a = None
else:
    class Selected:
        selected_b = None

selected_instance = Selected()


def read_selected(name):
    return getattr(selected_instance, name)


class Descriptors:
    @classmethod
    def build(cls, value):
        return value

    @staticmethod
    def make(value):
        return value

    @property
    def shown(self):
        return self

    @classmethod
    @replace
    def stacked(cls, value):
        return value


class Shadowed:
    classmethod = replace

    @classmethod
    def build(cls, value):
        return value


class Ignoring:
    @classmethod
    def ignores(cls, value, unused):
        return value


@classmethod
def loose(cls, value):
    return value


class Settable:
    @property
    def level(self):
        return self._level

    @level.setter
    def level(self, value):
        self._level = value

    @level.deleter
    def level(self):
        del self._level
