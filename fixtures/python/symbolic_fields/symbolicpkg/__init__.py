"""Exact record source controls; input data, never executed."""
import dataclasses as records
from dataclasses import dataclass as record, field

@record(kw_only=True)
class KeywordOptions:
    required: int
    optional: str = "default"

    def read(self):
        return self.required

@records.dataclass
class QualifiedOptions:
    value: int

    def read(self):
        return self.value

@record
class AliasOptions:
    value: int = field(default=7)

    def read(self):
        return self.value

@record(slots=True)
class ReplacedAllocation:
    value: int

    def read(self):
        return self.value

def opaque():
    return False

@record(init=opaque())
class DynamicOptions:
    value: int

class Impostor:
    @staticmethod
    def dataclass(cls):
        return cls

@Impostor.dataclass
class NonstandardTarget:
    value: int

@record
class DynamicDefault:
    value: int = opaque()

@record(init=False)
class ConditionalRecord:
    value: int

    def __init__(self, flag, incoming):
        if flag:
            self.value = incoming

    def read(self):
        return self.value

@record(init=False)
class DecoratedReader:
    value: int

    def __init__(self, incoming):
        self.value = incoming

    @opaque
    def read(self):
        return self.value

@record(init=False)
class RewrittenRecord:
    value: int

    def __init__(self, incoming):
        self.value = incoming
        self.value = 7

    def read(self):
        return self.value
