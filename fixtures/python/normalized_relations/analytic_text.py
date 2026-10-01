class Container:
    """An original source class."""

    async def méthode(self, value: int, /, *items, option=None, **options):
        """Keep the original parameter and documentation evidence."""
        return value


def factory():
    def nested(value="αβ"):
        """Nested source declaration."""
        return value

    return nested
