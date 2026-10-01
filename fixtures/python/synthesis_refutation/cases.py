"""Native input for the exact finite negative Summary question; never executed."""
from typing import TYPE_CHECKING

__all__ = ["false_source_control"]


def false_source_control(value):
    """Return the input only under the runtime-false TYPE_CHECKING condition."""
    return value if TYPE_CHECKING else None
