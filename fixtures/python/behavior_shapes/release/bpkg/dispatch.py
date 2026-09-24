"""Override dispatch: `self.render(...)` reaches whichever subclass overrides it."""


class Base:
    def handle(self, value):
        return self.render(value)

    def render(self, value):
        return value


class Derived(Base):
    def render(self, value):
        return str(value)
