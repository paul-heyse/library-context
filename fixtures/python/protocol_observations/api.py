
from typing import Any, NoReturn, Literal, overload
from types import TracebackType
import socket

class Known:
    def __enter__(self) -> None: ...
    def __exit__(self, a: type[BaseException] | None, b: BaseException | None, c: TracebackType | None) -> Literal[False]: ...

class Overloaded:
    def __enter__(self) -> None: ...
    @overload
    def __exit__(self, a: None, b: None, c: None) -> Literal[False]: ...
    @overload
    def __exit__(self, a: type[BaseException], b: BaseException, c: TracebackType) -> Literal[True]: ...
    def __exit__(self, a: Any, b: Any, c: Any) -> bool: ...

class NormalOnly:
    def __enter__(self) -> None: ...
    def __exit__(self, a: None, b: None, c: None) -> Literal[False]: ...

class AsyncGood:
    async def __aenter__(self) -> None: ...
    async def __aexit__(self, a: Any, b: Any, c: Any) -> Literal[False]: ...

class AsyncBad:
    async def __aenter__(self) -> None: ...
    def __aexit__(self, a: Any, b: Any, c: Any) -> bool: ...

def exits(gradual: Any) -> None:
    with Known(): pass
    with Overloaded(): pass
    with NormalOnly(): pass
    with gradual: pass

async def async_exits() -> None:
    async with AsyncGood(): pass
    async with AsyncBad(): pass

def declared() -> NoReturn:
    raise RuntimeError

def inferred():
    raise RuntimeError

class Method:
    def declared_method(self) -> NoReturn:
        raise RuntimeError
    def inferred_method(self):
        raise RuntimeError
    def placeholder(self):
        raise NotImplementedError

class Concrete(Method):
    def inferred_method(self) -> None: pass
    def placeholder(self) -> None: pass

def declared_use() -> None:
    declared()
    print('declared_dead')

def inferred_use() -> None:
    inferred()
    print('inferred_dead')

def declared_method_use(m: Method) -> None:
    m.declared_method()
    print('declared_method_dead')

def inferred_method_use(m: Method) -> None:
    m.inferred_method()
    print('inferred_method_live')

def placeholder_use(m: Method) -> None:
    m.placeholder()
    print('placeholder_live')

def receiver_narrowed_away(af: int, sa: object) -> None:
    sock = None
    try:
        sock = socket.socket(af)
        return
    except OSError:
        if sock is not None:
            sock.close()
            sock = None
