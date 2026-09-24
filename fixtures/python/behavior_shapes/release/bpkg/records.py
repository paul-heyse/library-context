"""Class facets: a dataclass with no public `__init__`, an explicit `__init__`, an inherited one."""

from dataclasses import dataclass


@dataclass
class Point:
    x: int
    name: str = ""


class Plain:
    def __init__(self, name, size=0):
        self.name = name
        self.size = size


class Configured(Plain):
    pass
