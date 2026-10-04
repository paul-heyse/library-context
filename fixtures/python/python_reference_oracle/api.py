from typing import TYPE_CHECKING

def decorate(function):
    return function

@decorate
def operation(value: int) -> int:
    copy = value
    copy += 1
    return copy

reference = operation
result = operation(value=1)

def parameter_control(value: int) -> int:
    return value

parameter_result = parameter_control(value=2)

def shadow(operation):
    return operation

if TYPE_CHECKING:
    typed_reference = operation

if False:
    unreachable_reference = operation

def unused():
    pass

class Resource:
    @property
    def property_value(self) -> int:
        return 1

resource = Resource()
property_read = resource.property_value
