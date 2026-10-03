"""Forced coarse failures over an actual generation service and original native grants.

Only the domain operation outcome is forced. Native admission/CPU serialization/budget and
shutdown remain real, and each failure envelope is admitted before the pinned writer sees it.
"""

import json
from dataclasses import replace
from pathlib import Path

from lctx_storage import StorageError

KINDS = ("resource_refused", "incompatible", "corrupt", "unavailable")


def install(receipt=None, *, fallback=False):
    import lctx_mcp.server as owner

    original = owner.open_generation
    observed: list[dict[str, str | bool]] = []

    class Service:
        def __init__(self, actual):
            self.actual = actual
            self.tool = self.resource = 0
            self.failed = set()

        def __getattr__(self, name):
            return getattr(self.actual, name)

        def failure(self, grant, index):
            self.failed.add(id(grant))
            exc = StorageError("SECRET postgres://password@private/driver/path")
            exc.kind = KINDS[index % len(KINDS)]
            return exc

        async def dispatch(self, grant, *args, **kwargs):
            index = self.tool
            self.tool += 1
            raise self.failure(grant, index)

        async def capability_resource(self, grant, *args):
            index = self.resource
            self.resource += 1
            raise self.failure(grant, index)

        async def encode_envelope(self, grant, encode, expanded):
            def capture():
                stdio, http = encode()
                if b"lctx_failure" in stdio or b'"error"' in stdio:
                    assert id(grant) in self.failed, "must use the original failing grant"
                    if fallback:
                        observed.append({"refused": True})
                        exc = StorageError("SECRET envelope driver text")
                        exc.kind = "resource_refused"
                        raise exc
                    observed.append({"stdio": stdio.hex(), "http": http.hex()})
                return stdio, http

            result = await self.actual.encode_envelope(grant, capture, expanded)
            if receipt is not None:
                Path(receipt).write_text(json.dumps(observed))
            return result

    async def opened(*args, **kwargs):
        served = await original(*args, **kwargs)
        return replace(served, service=Service(served.service))

    owner.open_generation = opened
    return observed
