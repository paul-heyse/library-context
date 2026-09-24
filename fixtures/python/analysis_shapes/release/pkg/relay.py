"""Outside the analytics config's module prefixes (the ADR set's re-review R5): a public function
that forwards its parameter into another release function, so the behavior scan's reach beyond the
subsystem has a known answer."""

from pkg.registry import Catalog


def relay(path: str) -> None:
    """Load a catalog through a fresh one."""
    Catalog().load(path)
