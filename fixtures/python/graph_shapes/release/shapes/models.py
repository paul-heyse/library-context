from depmod import DepBase


class Plain:
    """A class without bases."""


class Base(DepBase):
    def run(self):
        return 1


class Child(Base):
    # An override chain: Child.run -> Base.run -> depmod.DepBase.run.
    def run(self):
        return super().run() + 1
