class Box[T](list[T]):
    value = T

type Alias[T, U: T] = list[U]

def parameter[T](value: T) -> T:
    return value

def attached(value):
    return value

def undefined():
    return missing

def deleted():
    value = 1
    del value
    return value

def nested(value):
    def inner():
        nonlocal value
        value = 1
    return value

def pruned(flag):
    value = 1
    if flag:
        value = 2
    if not flag:
        return value

def looping(values):
    value = 0
    for element in values:
        value = value + element
        observed = value
    return value

from typing import TYPE_CHECKING

def pruned_runtime():
    if TYPE_CHECKING:
        value = 1
    else:
        value = 2
    return value

def unavailable_alternative(value):
    match value:
        case ("a", observed) | ("b", observed):
            return observed
    return value
