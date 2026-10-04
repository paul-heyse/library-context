from dataclasses import dataclass
from typing import NamedTuple, TypedDict


class Plain:
    pass


@dataclass
class Data:
    value: int


class TupleRecord(NamedTuple):
    value: int


class DictRecord(TypedDict):
    value: int


FunctionalTuple = NamedTuple("FunctionalTuple", [("value", int)])
FunctionalDict = TypedDict("FunctionalDict", {"value": int})
