"""Run the external-review probes under sys.monitoring: which `emit()` lines execute."""
import sys, types

SRC = '''import contextlib
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
'''
code = compile(SRC, "m.py", "exec")
m = types.ModuleType("m"); exec(code, m.__dict__)
lines = set()
mon = sys.monitoring
TOOL = mon.PROFILER_ID
mon.use_tool_id(TOOL, "probe")
def on_line(c, line):
    if c.co_filename == "m.py":
        lines.add(line)
mon.register_callback(TOOL, mon.events.LINE, on_line)
mon.set_events(TOOL, mon.events.LINE)
answers = iter([True, False])
m.impure(lambda: next(answers), lambda: None)
print("two_ranges(2, 0) returns", m.two_ranges(2, 0), "(the loop-1 value reaches the return)")
class S:
    x = None
    def reset(self): self.x = 1
m.mutated(S(), lambda: None)
m.cleared([1], lambda: None)
m.eq_vs_is(1, lambda: None)
class Weird:
    def __eq__(self, other): return True
m.eq_none(Weird(), lambda: None)
m.version(lambda: None)
m.choose(True, lambda: None)
mon.set_events(TOOL, 0); mon.free_tool_id(TOOL)
emit_lines = [i + 1 for i, t in enumerate(SRC.splitlines()) if t.strip() == "emit()"]
print(sys.version.split()[0], "executed emit() lines:", sorted(l for l in emit_lines if l in lines), "of", emit_lines)
