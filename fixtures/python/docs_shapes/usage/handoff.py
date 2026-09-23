"""Pass C's known-answer shapes (DESIGN §9.3): a named handoff after a receiver use, a nested
one, and two that are none (two consumers; a reassigned binding)."""

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
