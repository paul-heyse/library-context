"""FCA's known-answer shapes (DESIGN §9.6): a family of registration methods that share
parameters, parameter types and a return type, and one method that shares nothing. The
increment-2 review's shapes: two loaders whose return type Pyrefly cannot determine and which raise
one class both ways (F2), and a subclass that inherits the family and adds to it, a second scope
holding the same methods (F1)."""


class Catalog:
    def add_tool(
        self, fn, *, name: str | None = None, tags: set[str] | None = None, title: str | None = None
    ) -> None:
        """Register a tool."""

    def add_prompt(
        self, fn, *, name: str | None = None, tags: set[str] | None = None, title: str | None = None
    ) -> None:
        """Register a prompt."""

    def add_resource(
        self, uri: str, fn, *, name: str | None = None, tags: set[str] | None = None
    ) -> None:
        """Register a resource."""

    def remove(self, key: int) -> bool:
        """Remove a component."""
        if key < 0:
            raise KeyError(key)
        return True

    def load(self, path: str) -> Missing:
        """Load a catalog."""
        if not path:
            raise KeyError(path)

    def reload(self, path: str) -> Missing:
        """Reload a catalog."""
        if not path:
            raise KeyError


class Widgets(Catalog):
    def add_gadget(
        self, fn, *, name: str | None = None, tags: set[str] | None = None, title: str | None = None
    ) -> None:
        """Register a gadget."""

    def add_widget(
        self, fn, *, name: str | None = None, tags: set[str] | None = None, title: str | None = None
    ) -> None:
        """Register a widget."""
