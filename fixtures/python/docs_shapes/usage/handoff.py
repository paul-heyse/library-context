"""Pass C's known-answer shapes (DESIGN §9.3): a named handoff after a receiver use, a nested
one, and five that are none: two consumers; a reassigned binding; a helper this code defines; a
chained assignment; an attribute read passed on (slice 2.2 review F2, F4)."""

from pkg import make_server

primary = make_server("primary")
helper = make_server("helper")
helper.run()
primary.tool(helper)
primary.tool(make_server("nested"))

twice = make_server("twice")
primary.tool(twice)
primary.tool(twice)

rebound = make_server("a")
rebound = make_server("b")
primary.tool(rebound)


def local_factory():
    return make_server("local")


primary.tool(local_factory())

a = b = make_server("chain")
primary.tool(a)

named = make_server("named")
print(named.run)
primary.tool(named)
