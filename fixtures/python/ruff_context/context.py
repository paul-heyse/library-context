from typing import TYPE_CHECKING as TC
from typing import overload as ov
from typing import final as fin

if TC:
    typing_value = 1

@ov
def typed(x: int) -> int: ...

@ov
def typed(x: str) -> str: ...

def typed(x):
    return x

@fin
class Closed:
    pass

def overload(function):
    return function

@overload
def ordinary(x):
    return x

fin = 0
fin
