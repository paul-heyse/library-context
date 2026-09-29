"""Catalog-only contract fixture; input data, never executed by extraction."""
from dataclasses import dataclass
from typing import overload

def ordinary(first: int, /, optional: str = "ok", *items: int, flag: bool = False, **options: str) -> str:
    """Return a value using the declared controls.

    Args:
        optional: The original optional text.
        flag: A keyword-only switch.
    """
    return optional

alias = ordinary

def undocumented():
    pass

def wrapper(function):
    return function

@wrapper
def wrapped(value: int = None):
    return value

@overload
def choose(value: int, /) -> int: ...
@overload
def choose(value: str, *, prefix: str = "") -> str: ...
def choose(value, **options):
    return value

class Base:
    def method(self, *, count: int = 1):
        return count

class Child(Base):
    pass

@dataclass
class Config:
    count: int
    title: str = "sample"

class ChildConfig(Config):
    pass

@dataclass
class _PrivateConfig:
    value: int

class PublicConfig(_PrivateConfig):
    pass

from dataclasses import field
from typing import NamedTuple, TypedDict, NotRequired
from functools import cached_property, wraps
from contextlib import contextmanager

@dataclass
class Options:
    left: str = 'http'
    right: int = 0x10
    cache: list[str] = field(default_factory=list)

    def read_left(self):
        return self.left

    def read_right(self):
        return self.right

    @staticmethod
    def unrelated(value):
        return value.left

@dataclass(init=False)
class DirectOptions:
    value: int

    def __init__(self, incoming: int):
        self.value = incoming

    def read(self):
        return self.value

@dataclass
class CustomAllocation:
    value: int

    def __new__(cls, value):
        return object.__new__(cls)

@dataclass
class CustomMeta(metaclass=type):
    value: int

@wrapper
@dataclass
class ReplacedRecord:
    value: int

class Pair(NamedTuple):
    name: str
    count: int = 1

class MappingOptions(TypedDict):
    value: int
    label: NotRequired[str]

class Descriptors:
    @property
    def value(self):
        return 1

    @value.setter
    def value(self, incoming):
        pass

    @cached_property
    def cached(self):
        return 'cached'

    @classmethod
    def create(cls):
        return cls()

@contextmanager
def managed():
    yield 1

@wraps(ordinary)
def wrapped_signature(*args, **kwargs):
    return ordinary(*args, **kwargs)

@dataclass
class DescriptorField:
    value: int

    @property
    def value(self):
        return 99

    @value.setter
    def value(self, incoming):
        self._other = incoming + 1

@dataclass(init=False)
class SetterMutation:
    value: int

    def __init__(self, incoming: int):
        self.value = incoming
        self.trigger = incoming

    @property
    def trigger(self):
        return 0

    @trigger.setter
    def trigger(self, incoming):
        self.value = 99

@dataclass(init=False)
class OperatorMutation:
    value: int
    other: int

    def __init__(self, incoming: int, opaque):
        self.value = incoming
        self.other = opaque + 1

class ReplacedProperty:
    @opaque
    @property
    def value(self):
        return 1

    @value.setter
    def value(self, incoming):
        self._value = incoming

from builtins import classmethod as class_binding
from fastmcp import FastMCP

class AliasDescriptor:
    @class_binding
    def create(cls):
        return cls()

registry = FastMCP("source-only fixture")

@registry.tool
def registered(value: str):
    return value

@registry.resource("sample://entry")
def registered_resource():
    return "entry"

@registry.prompt
def registered_prompt():
    return "prompt"

@dataclass(init=False)
class StaticConstructor:
    value: int

    @staticmethod
    def __init__(receiver, incoming: int):
        receiver.value = incoming

from typing import Literal

def literal_controls(text: Literal["http", "stdio"], flag: Literal[True], count: Literal[7]):
    return text

# Rebinding a public class does not change the owner of a retained old alias.
@dataclass
class ReboundOptions:
    old_field: int = 1

RetainedOptions = ReboundOptions

@dataclass
class ReboundOptions:
    new_field: str = "new"
