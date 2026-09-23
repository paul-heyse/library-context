"""Stage C/D edge cases (slice-2 review F1, F2): synthesized members and binding choices."""

from dc.impl import Holder, Point, make, wrapped
from dc.compat import load

__all__ = ["Holder", "Point", "api", "load", "make", "wrapped"]


def api(x):
    Point(1, 2)
    Holder().fn(1)
    return make(x)
