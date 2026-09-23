"""Flow and outside names (C3 review F1, F3-F6): a module with no star import."""

from typing import Annotated

print(__name__, __file__, __doc__)  # the module's implicit globals

r = len  # read before the module binds `len`: that binding or the builtin
len = 5  # noqa: A001


class Model:
    a = type  # read before the class binds `type`
    type = "k"  # noqa: A003
    tags: Annotated[list[str], (lambda: [])] = []  # nothing opens inside an annotation

    def which(self):
        return __class__  # the method's `__class__` cell


def missing():
    return Callable  # noqa: F821  never imported: unresolved, not a builtin


def lazy():
    global json_mod
    import json as json_mod


def use_lazy():
    return json_mod


def order():
    def inner():
        nonlocal v
        v = 2

    v = 1
    return inner, v


def decorate(f):
    return f


def outer_scope(v):
    @decorate(lambda: v)
    def inner(w=[v for _ in range(1)]):
        return w

    return inner


def twice():
    global g1, g1
    g1 = 1
