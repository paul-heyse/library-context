"""Owned, transparent gRPC response faults; never an operator endpoint or product hook."""

import asyncio
import base64
import contextlib
import json
import os
import urllib.request
from pathlib import Path
from urllib.parse import urlsplit


class ResponseGate:
    """Forward the actual SDK protocol, optionally delaying a received DATA frame."""

    def __init__(self, endpoint):
        self.upstream = urlsplit(endpoint)
        self.server = None
        self.tasks = set()
        self.writers = set()
        self.entered = asyncio.Event()
        self.released = asyncio.Event()
        self.released.set()
        self.unavailable = False

    async def __aenter__(self):
        self.server = await asyncio.start_server(self._connection, "127.0.0.1", 0)
        port = self.server.sockets[0].getsockname()[1]
        self.endpoint = f"{self.upstream.scheme}://127.0.0.1:{port}"
        return self

    def pause(self):
        self.entered.clear()
        self.released.clear()

    def resume(self):
        self.unavailable = False
        self.released.set()

    def disconnect(self):
        self.unavailable = True
        self.released.set()
        for writer in tuple(self.writers):
            writer.close()

    async def _connection(self, incoming, client):
        task = asyncio.current_task()
        self.tasks.add(task)
        upstream = None
        forwarding = []
        try:
            if self.unavailable:
                return
            reader, upstream = await asyncio.open_connection(
                self.upstream.hostname, self.upstream.port
            )
            self.writers.update([client, upstream])

            async def requests():
                while data := await incoming.read(65536):
                    upstream.write(data)
                    await upstream.drain()

            async def responses():
                # HTTP/2 server frames have a nine-byte header. No gRPC payload is decoded,
                # replaced or fabricated; pause only after an actual response DATA frame.
                while True:
                    header = await reader.readexactly(9)
                    payload = await reader.readexactly(int.from_bytes(header[:3], "big"))
                    if header[3] == 0 and not self.released.is_set():
                        self.entered.set()
                        await self.released.wait()
                    client.write(header + payload)
                    await client.drain()

            forwarding = [asyncio.create_task(requests()), asyncio.create_task(responses())]
            await asyncio.wait(forwarding, return_when=asyncio.FIRST_COMPLETED)
        except OSError, asyncio.IncompleteReadError:
            pass
        finally:
            for child in forwarding:
                child.cancel()
            await asyncio.gather(*forwarding, return_exceptions=True)
            for writer in [client, upstream]:
                if writer is not None:
                    self.writers.discard(writer)
                    writer.close()
                    with contextlib.suppress(OSError):
                        await writer.wait_closed()
            self.tasks.discard(task)

    async def __aexit__(self, *exc):
        self.resume()
        assert self.server is not None
        self.server.close()
        await self.server.wait_closed()
        for task in tuple(self.tasks):
            task.cancel()
        await asyncio.gather(*tuple(self.tasks), return_exceptions=True)


def viewer_config(configured, endpoint, target):
    config = json.loads(Path(configured).read_text())
    config["endpoint"] = endpoint
    target.write_text(json.dumps(config))
    target.chmod(0o600)
    return target


def fixture_query(selected, sql):
    """Administrative fault injection only on the explicitly owned test fixture."""
    path = os.environ.get("LCTX_SURREAL_TEST_CONFIG")
    assert path, "owned persistent fixture configuration required"
    config = json.loads(Path(path).read_text())
    credentials = f"{config['admin_user']}:{config['admin_password']}"
    request = urllib.request.Request(
        config["endpoint"] + "/sql",
        data=sql.encode(),
        headers={
            "Authorization": "Basic " + base64.b64encode(credentials.encode()).decode(),
            "Surreal-NS": selected["database"]["namespace"],
            "Surreal-DB": selected["database"]["database"],
            "Accept": "application/json",
            "Content-Type": "text/plain",
        },
    )
    with urllib.request.urlopen(request, timeout=20) as response:
        rows = json.load(response)
    assert isinstance(rows, list) and all(row["status"] == "OK" for row in rows), (
        "owned fault injection failed"
    )
    return rows
