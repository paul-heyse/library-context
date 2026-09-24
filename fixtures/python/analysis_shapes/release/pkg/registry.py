"""FCA's known-answer shapes (DESIGN §9.6): a family of registration methods that share
parameters, parameter types and a return type, and one method that shares nothing."""


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
