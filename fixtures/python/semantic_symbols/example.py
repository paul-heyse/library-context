from typing import Generic, TypeVar

T = TypeVar("T")

__all__ = ["Service", "Missing"]


class Base:
    def run(self, timeout: float = 1.0) -> None:
        """Run once.

        Args:
            timeout: Seconds to wait.
        """


class Service(Base, Generic[T]):
    @staticmethod
    def make() -> "Service[int]":
        return Service()

    def run(self, timeout: float = 1.0) -> None:
        def inner() -> None: ...
        inner()
