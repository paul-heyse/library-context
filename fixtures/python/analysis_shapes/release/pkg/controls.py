"""Pass B's known-answer shapes (DESIGN §9.2, §12): forwarding directly and through one alias,
two mappings of one parameter, a literal, a bound receiver, a class receiver, a constructor, a
depth-2 chain and the depth bound, `**kwargs`, a positional after a starred argument, a guard
behind a `try` or a suppressing `with`, a guard its caller's branch contradicts, a call made on
some paths only, a guard on an attribute, a guard after rebinding, and a rebound or computed
value (slice 2.1 review F1, F4, F5, F6)."""

import contextlib


class Registry:
    def __init__(self):
        self.items = []

    def add(self, item, *, tag=None):
        if item is None:
            raise TypeError("an item is required")
        self.items.append(item)
        return item, tag

    @classmethod
    def create(cls, capacity):
        if capacity > 100:
            raise ValueError("too large")
        return cls()


class Widget:
    def __init__(self, label):
        if label is None:
            raise TypeError("a label is required")
        self.label = label


def configure(name, size=10, *extra, mode="fast", note=None, **options):
    """Configure a component.

    Args:
        name: The component's name; it must not
            be empty.
        size: How many slots to reserve.
        mode: The speed mode.
        note: A note to annotate with.
    """
    alias = name
    build(alias, size, flag=True)
    swap(size, name)
    swap(name, size)
    swap(*extra, name)
    validate(mode)
    Registry().add(name, tag=mode)
    Registry.create(size)
    Widget(name)
    try:
        checked(size)
    except ValueError:
        pass
    with contextlib.suppress(ValueError):
        muted(size)
    if name is not None:
        strict(name)
    if mode == "slow":
        slow(name)
    passthrough(**options)
    rebinding(name)
    note = note or name
    annotate(note)
    annotate(name.upper())
    probe_attr(name)

    # Defined and called once: one call site, whatever the definition arc adds.
    def audit():
        return name

    audit()
    return extra


def build(label, count, flag=False):
    if not label:
        raise ValueError("a label is required")
    return finish(label, count), flag


def finish(text, count):
    if len(text) > count:
        raise ValueError("too long")
    return record(text)


def record(entry):
    return entry


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


def muted(count):
    if count < 0:
        raise ValueError(count)
    return count


def strict(value):
    if value is None:
        raise ValueError("a value is required")
    return value


def slow(label):
    if len(label) > 64:
        raise ValueError("too long")
    return label


def passthrough(**options):
    return options


def rebinding(label):
    label = label.strip()
    if not label:
        raise ValueError("empty")
    return label


def annotate(text):
    return text


def probe_attr(obj):
    if obj.closed:
        raise RuntimeError("closed")
    return obj
