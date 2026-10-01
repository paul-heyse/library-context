"""Independent placeholder/concrete twins retained from dormant P4 tests; never executed."""
from abc import abstractmethod as abstract_alias
from typing import Protocol


class Abstract:
    @abstract_alias
    def run(self, unused): ...


class Interface(Protocol):
    def dispatch(self, unused): ...


class Concrete:
    def run(self, unused):
        return 1

    def not_implemented_value(self, unused):
        return NotImplemented
