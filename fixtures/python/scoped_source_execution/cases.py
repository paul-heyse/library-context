"""Static input for the bounded SourceCall and Enriched oracle control."""


def literal_argument():
    def fresh(item):
        return item
    fresh(7)
    return 1


def held_argument(value):
    def fresh(item):
        return item
    fresh(value)
    return value


def documented_argument():
    """Exact Unicode: café λ 😀."""
    def fresh(item):
        return item
    fresh(11)
    return 2


def unavailable_default():
    def fresh(item=5):
        return item
    fresh()
    return 3
