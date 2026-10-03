from .base import run as known_alias, run as _private_alias

def dynamic_names() -> list[str]:
    return ["other"]

__all__ = ["known_alias", "_private_alias"]
__all__.extend(dynamic_names())

def fallback_only() -> int:
    return 0
