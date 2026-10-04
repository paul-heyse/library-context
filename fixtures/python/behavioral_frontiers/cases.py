from typing import Any, Callable, Literal, NoReturn, Never, Protocol, final
from types import TracebackType

def declared() -> NoReturn:
    raise RuntimeError

def inferred():
    raise RuntimeError

class Open:
    def stop(self) -> NoReturn:
        raise RuntimeError
    def inferred(self):
        raise RuntimeError
    def placeholder(self):
        raise NotImplementedError

class Override(Open):
    def inferred(self) -> None:
        pass
    def placeholder(self) -> None:
        pass

@final
class Final:
    def stop(self) -> NoReturn:
        raise RuntimeError

class Shape(Protocol):
    def stop(self) -> NoReturn: ...

def declared_use() -> None:
    declared()
    print("normal edge remains unconditional elsewhere")

def inferred_use() -> None:
    inferred()
    print("unknown inference")

def open_use(obj: Open) -> None:
    obj.stop()
    print("overridable")

def final_use(obj: Final) -> None:
    obj.stop()
    print("typing final is conditional")

def protocol_use(obj: Shape) -> None:
    obj.stop()
    print("no runtime protocol closure")

def higher_order_use(fn: Callable[[], NoReturn]) -> None:
    fn()
    print("potential target")

def never_use(obj: Never) -> None:
    obj.stop()
    print("uninhabited receiver")

def guarded_use() -> None:
    try:
        declared()
    finally:
        print("cleanup is mandatory")
    print("no direct suite frontier through cleanup")

class Known:
    def __enter__(self) -> None: ...
    def __exit__(self, a: type[BaseException] | None, b: BaseException | None, c: TracebackType | None) -> Literal[False]: ...

class Async:
    async def __aenter__(self) -> None: ...
    async def __aexit__(self, a: Any, b: Any, c: Any) -> Literal[False]: ...

class LiteralSuppress:
    def __enter__(self) -> None: ...
    def __exit__(self, a: Any, b: Any, c: Any) -> Literal[True]: ...

class NoneExit:
    def __enter__(self) -> None: ...
    def __exit__(self, a: Any, b: Any, c: Any) -> None: ...

def exits(unknown: Any) -> None:
    with Known(): pass
    with LiteralSuppress(): pass
    with NoneExit(): pass
    with unknown: pass

async def async_exits() -> None:
    async with Async(): pass

def operators(xs: list[int], x: int) -> None:
    x + 1
    for value in xs:
        repr(value)
    gen = (value for value in xs)
    next(gen)

class FinalMethod:
    @final
    def stop(self) -> NoReturn:
        raise RuntimeError

def final_method_use(obj: FinalMethod) -> None:
    obj.stop()
    print("method final is conditional")

@final
class Placeholder:
    def stop(self) -> NoReturn:
        raise NotImplementedError

def placeholder_use(obj: Placeholder) -> None:
    obj.stop()
    print("placeholder remains unknown")

@final
class Property:
    @property
    def stop(self) -> Callable[[], NoReturn]:
        return declared

def property_use(obj: Property) -> None:
    obj.stop()
    print("property has no native member body certificate")

class RuntimeOverride(Final):
    def stop(self) -> None:
        pass

def override_control() -> None:
    final_use(RuntimeOverride())

class MroLeft: pass
class MroRight: pass
class MroLR(MroLeft, MroRight): pass
class MroRL(MroRight, MroLeft): pass
class MroGap(MroLR, MroRL):
    @final
    def stop(self) -> NoReturn:
        raise RuntimeError

def mro_gap_use(obj: MroGap) -> None:
    obj.stop()
    print("incomplete ancestry remains open")

async def async_declared() -> NoReturn:
    raise RuntimeError

def async_creation_use() -> None:
    async_declared()
    print("coroutine creation does not enter its body")

def generator_declared() -> NoReturn:
    yield 1
    raise RuntimeError

def generator_creation_use() -> None:
    generator_declared()
    print("generator creation does not enter its body")
