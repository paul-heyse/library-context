"""Usage a pattern must not be cut from (slice 2.2 review F1): a call inside a `with` whose
header changes what it does, and a call reading a name nothing binds."""

import contextlib

from pkg import make_server

server = make_server("s")
with contextlib.suppress(TypeError):
    server.stop()
server.stop(undefined_timeout)
