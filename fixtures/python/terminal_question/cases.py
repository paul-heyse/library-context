from typing import Callable, Never, NoReturn, Protocol, final

__all__ = [
    'declared_use', 'final_use', 'final_method_use', 'open_use', 'protocol_use',
    'higher_order_use', 'never_use', 'property_use', 'placeholder_use',
    'inferred_use', 'cleanup_use', 'async_creation_use', 'mro_gap_use',
]

def stop() -> NoReturn:
    raise RuntimeError

@final
class Final:
    def stop(self) -> NoReturn:
        raise RuntimeError

class FinalMethod:
    @final
    def stop(self) -> NoReturn:
        raise RuntimeError

class Open:
    def stop(self) -> NoReturn:
        raise RuntimeError

class Shape(Protocol):
    def stop(self) -> NoReturn: ...

@final
class Placeholder:
    def stop(self) -> NoReturn:
        raise NotImplementedError

@final
class Property:
    @property
    def stop(self) -> Callable[[], NoReturn]:
        return stop

def inferred():
    raise RuntimeError

async def async_stop() -> NoReturn:
    raise RuntimeError

class MroLeft: pass
class MroRight: pass
class MroLR(MroLeft, MroRight): pass
class MroRL(MroRight, MroLeft): pass
class MroGap(MroLR, MroRL):
    @final
    def stop(self) -> NoReturn:
        raise RuntimeError

def declared_use() -> None:
    """Return control after invoking the declared operation."""
    stop()
    print('following')

def final_use(obj: Final) -> None:
    """Return control after invoking the final class member."""
    obj.stop()
    print('following')

def final_method_use(obj: FinalMethod) -> None:
    """Return control after invoking the final method."""
    obj.stop()
    print('following')

def open_use(obj: Open) -> None:
    """Return control after an open receiver call."""
    obj.stop()
    print('following')

def protocol_use(obj: Shape) -> None:
    """Return control after a protocol call."""
    obj.stop()
    print('following')

def higher_order_use(fn: Callable[[], NoReturn]) -> None:
    """Return control after a higher order call."""
    fn()
    print('following')

def never_use(obj: Never) -> None:
    """Return control after an uninhabited receiver call."""
    obj.stop()
    print('following')

def property_use(obj: Property) -> None:
    """Return control after a property call."""
    obj.stop()
    print('following')

def placeholder_use(obj: Placeholder) -> None:
    """Return control after a placeholder call."""
    obj.stop()
    print('following')

def inferred_use() -> None:
    """Return control after an inferred call."""
    inferred()
    print('following')

def cleanup_use() -> None:
    """Return control after mandatory cleanup."""
    try:
        stop()
    finally:
        print('cleanup')
    print('following')

def async_creation_use() -> None:
    """Return control after creating a coroutine."""
    async_stop()
    print('following')

def mro_gap_use(obj: MroGap) -> None:
    """Return control after a receiver with inconsistent ancestry."""
    obj.stop()
    print('following')
