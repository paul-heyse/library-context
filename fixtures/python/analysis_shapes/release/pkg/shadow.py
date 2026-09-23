import extdep

from pkg.helpers import helper


class Mixin:
    def run(self):
        return helper(self)

    def tool(self):
        return 1


# `run` may come from the dependency's base: a seed through it must refuse, not pick Mixin.run.
class Shadowed(extdep.Base, Mixin):
    pass


# `tool` is rebound by an assignment in the class body: a seed must refuse, not pick Mixin.tool.
class Aliased(Mixin):
    tool = helper
