from typing import Callable, TypeVar, overload

T = TypeVar("T")


def plain(x: int, y: int = 2) -> int:
    return x + y


def wrapper(fn: Callable[[int], int]) -> Callable[[int], int]:
    return fn


@wrapper
def decorated(x: int) -> int:
    return x


@overload
def variant(x: int) -> int: ...
@overload
def variant(x: int, y: int) -> int: ...
def variant(x: int, y: int = 0) -> int:
    return x + y


def run(items: list[int]) -> None:
    plain(1)
    plain(1, 2, 3)
    plain(*items)
    decorated(1)
    variant(1)


class ReceiverOwner:
    @classmethod
    def class_call(cls, x: int) -> int:
        return x

    def instance_call(self, x: int) -> int:
        return x

    @staticmethod
    def static_call(x: int) -> int:
        return x


class OpenReceiverOwner:
    @classmethod
    def class_call(cls, x: int) -> int:
        return x


class ReceiverLeft(ReceiverOwner):
    @classmethod
    def class_call(cls, x: int) -> int:
        return x


class ReceiverRight(ReceiverOwner):
    @classmethod
    def class_call(cls, x: int) -> int:
        return x


class ReceiverDiamond(ReceiverLeft, ReceiverRight):
    @classmethod
    def class_call(cls, x: int) -> int:
        return x


def receiver_twins(obj: ReceiverOwner, open_obj: OpenReceiverOwner) -> None:
    ReceiverOwner.class_call(1)
    obj.class_call(1)
    obj.instance_call(1)
    obj.static_call(1)
    open_obj.class_call(1)
