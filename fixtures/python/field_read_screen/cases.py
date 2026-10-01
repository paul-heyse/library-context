"""Source-model read-screen input; never executed."""


class Cold:
    declared = None

    def __init__(self, value):
        self.unread = value


class Hot:
    def __init__(self, value):
        self.loaded = value

    def read(self):
        return self.loaded


class Dynamic:
    def __init__(self, value):
        self.reachable = value

    def lookup(self, name):
        return getattr(self, name)


class Child(Dynamic):
    def __init__(self, value):
        self.child_field = value


fixed = Cold(None)


def ignored(value, unused):
    return value


from builtins import hasattr as named_probe
import builtins


class Named:
    def __init__(self):
        self.literal_seen = None
        self.alias_seen = None


def named_reads(value):
    named_probe(value, "alias_seen")
    return builtins.getattr(value, "literal_seen")
