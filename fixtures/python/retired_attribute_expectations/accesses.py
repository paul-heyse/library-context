"""Exact resolved-builtin spellings and computed-name negatives; never executed."""
import builtins
from builtins import getattr as read_attribute


class Settings:
    def __init__(self):
        self.bare_only = 1
        self.qualified_only = 2
        self.aliased_only = 3
        self.direct_only = 4
        self.unread_only = 5

    def bare(self):
        return getattr(self, "bare_only")

    def qualified(self):
        return builtins.getattr(self, "qualified_only")

    def aliased(self):
        return read_attribute(self, "aliased_only")

    def direct(self):
        return self.direct_only

    def shadowed(self):
        getattr = lambda obj, name: 0
        return getattr(self, "unread_only")


class DynamicSettings:
    def __init__(self):
        self.computed_name = 1

    def computed(self, suffix):
        return getattr(self, "computed_" + suffix)

    def formatted(self, suffix):
        return getattr(self, "{}".format(suffix))

    def joined(self, parts):
        return getattr(self, "".join(parts))
