from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from typing import Protocol, final, runtime_checkable, dataclass_transform, Any
from warnings import deprecated

class Meta(type): pass
class Custom(metaclass=Meta):
    __slots__ = ("stored",)

@final
@deprecated("use the typed interface")
class Closed: pass

@runtime_checkable
class Reader(Protocol):
    def read(self) -> str: ...

class Abstract(ABC):
    @abstractmethod
    def read(self) -> str: ...

class Concrete(Abstract):
    def read(self) -> str: return "ok"

@dataclass(frozen=True, kw_only=True, slots=True)
class Config:
    size: int = 3
    cache: list[int] = field(default_factory=list)

@dataclass_transform(kw_only_default=True, frozen_default=True)
class RecordBase: pass
class Transformed(RecordBase):
    value: int

class Choice(Enum):
    FIRST = 1
    SECOND = 2

class Plain: pass

class Access:
    def __init__(self) -> None:
        self.value = 1
    @property
    def computed(self) -> int:
        return self.value

class Derived(Access):
    @property
    def computed(self) -> int:
        return 2

def read(receiver: Access) -> int:
    assigned: int = "wrong"
    return receiver.value + receiver.computed

def dynamic(receiver: Any) -> Any:
    return receiver.value
