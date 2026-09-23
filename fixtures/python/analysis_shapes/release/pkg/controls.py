"""Pass B's known-answer shapes (DESIGN §9.2, §12): forwarding directly and through one alias,
two mappings of one parameter, a literal, a bound receiver, `**kwargs`, a guard behind a `try`
and a guard after rebinding."""


class Registry:
    def __init__(self):
        self.items = []

    def add(self, item, *, tag=None):
        if item is None:
            raise TypeError("an item is required")
        self.items.append(item)
        return item, tag


def configure(name, size=10, *extra, mode="fast", **options):
    """Configure a component.

    Args:
        name: The component's name; it must not
            be empty.
        size: How many slots to reserve.
        mode: The speed mode.
    """
    alias = name
    build(alias, size, flag=True)
    swap(size, name)
    swap(name, size)
    validate(mode)
    Registry().add(name, tag=mode)
    try:
        checked(size)
    except ValueError:
        pass
    passthrough(**options)
    rebinding(name)

    # Defined and called once: one call site, whatever the definition arc adds.
    def audit():
        return name

    audit()
    return extra


def build(label, count, flag=False):
    if not label:
        raise ValueError("a label is required")
    return label, count, flag


def swap(left, right):
    return right, left


def validate(mode):
    if mode not in ("fast", "slow"):
        raise ValueError(mode)
    return mode


def checked(count):
    if count < 0:
        raise ValueError(count)
    return count


def passthrough(**options):
    return options


def rebinding(label):
    label = label.strip()
    if not label:
        raise ValueError("empty")
    return label
