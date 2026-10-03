"""Two structural call clusters and an excluded community isolate."""
from typing import final
from dataclasses import dataclass
from warnings import deprecated

__all__ = ['alpha', 'beta', 'gamma', 'delta', 'epsilon', 'zeta', 'identity', 'isolate', 'LeftValue', 'RightValue']
@final
@dataclass(frozen=True)
class LeftValue:
    value: int
GLOBAL = 7

class RightValue:
    pass

def alpha(value: LeftValue, flag: bool = False) -> int:
    beta(gamma(value))
    gamma(value)
    return 1

def beta(value: LeftValue) -> int:
    alpha(value)
    gamma(value)
    return 2

def gamma(value: LeftValue) -> int:
    alpha(value)
    beta(value)
    return 3

def delta(value: RightValue) -> str:
    epsilon(value)
    zeta(value)
    return 'a'

def epsilon(value: RightValue) -> str:
    delta(value)
    zeta(value)
    return 'b'

def zeta(value: RightValue) -> str:
    delta(value)
    epsilon(value)
    return 'c'

def identity(value: int) -> int:
    return value

def _marker[T](func: T) -> T:
    return func

@deprecated("  use the next isolate  ")
@_marker
def isolate(value: int) -> int:
    return value + GLOBAL
