import contextlib
import sys


def impure(probe, emit):
    if probe():
        if not probe():
            emit()


def two_ranges(n, m):
    found = None
    for i in range(n):
        found = i
    for j in range(m):
        found = j
    return found


def mutated(self, emit):
    if self.x is None:
        self.reset()
        if self.x is not None:
            emit()


def cleared(items, emit):
    if items:
        items.clear()
        if not items:
            emit()


def eq_vs_is(x, emit):
    if x == True:
        if x is not True:
            emit()


def eq_none(x, emit):
    if x == None:
        if x is not None:
            emit()


def version(emit):
    if sys.version_info <= (3, 14):
        emit()
    if sys.version_info > (3, 14):
        emit()


def choose(TYPE_CHECKING, emit):
    if TYPE_CHECKING:
        emit()
