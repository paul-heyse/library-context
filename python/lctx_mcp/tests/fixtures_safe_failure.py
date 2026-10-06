"""Explicit injected executor for transport envelope controls, never native qualification."""

import sys
from functools import partial

import anyio
from fastmcp import FastMCP
from lctx_semantics import NativeFailure, wire_failure

from lctx_mcp.__main__ import run_stdio
from lctx_mcp.wire import EnvelopeAdmission, register


class InjectedFailure:
    async def execute(self, name, arguments):
        sentinel = "internal sentinel secret /private/failure/path"
        if self.kind == "unexpected":
            raise RuntimeError(sentinel)
        error = NativeFailure(sentinel)
        error.lctx_failure_json = wire_failure(self.kind)
        raise error

    def __init__(self, kind):
        self.kind = kind


def server(kind):
    result = FastMCP("injected failure envelope", cache_ttl=None, mask_error_details=True)
    result.add_middleware(EnvelopeAdmission(result))
    register(result, InjectedFailure(kind))
    return result


if __name__ == "__main__":
    anyio.run(partial(run_stdio, server(sys.argv[1])))
