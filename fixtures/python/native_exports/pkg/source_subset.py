def dynamic_names() -> list[str]:
    return []

__all__ = ["_retained_literal"] + dynamic_names()

def _retained_literal() -> int:
    return 1
