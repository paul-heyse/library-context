"""Finite tar I/O controls for the proposed cold-backup lowering; no service calls."""
from __future__ import annotations

import hashlib
import io
import json
import tarfile


class HashWriter:
    def __init__(self, stream, *, fail_at=None):
        self.stream, self.hash, self.count = stream, hashlib.sha256(), 0
        self.fail_at = fail_at

    def write(self, value):
        view = memoryview(value)
        while view:
            if self.fail_at is not None and self.count >= self.fail_at:
                raise OSError("injected write failure")
            part = view if self.fail_at is None else view[:self.fail_at - self.count]
            count = self.stream.write(part)
            if count is None or count <= 0:
                raise OSError("write made no progress")
            self.hash.update(part[:count])
            self.count += count
            view = view[count:]
        return len(value)

    def flush(self):
        self.stream.flush()


class ShortWriter(io.BytesIO):
    def write(self, value):
        return super().write(value[:37])


class HashReader:
    def __init__(self, data):
        self.source, self.hash = io.BytesIO(data), hashlib.sha256()

    def read(self, count=-1):
        data = self.source.read(count)
        self.hash.update(data)
        return data


def archive(files, stream, *, fail_at=None, fail_finalization=False, fail_fsync=False):
    writer = HashWriter(stream, fail_at=fail_at)
    inventory = {}
    with tarfile.open(fileobj=writer, mode="w|") as tar:
        for name, body in files.items():
            header = tarfile.TarInfo(name)
            header.size, header.mode = len(body), 0o600
            source = HashReader(body)
            tar.addfile(header, source)
            inventory[name] = {"sha256": source.hash.hexdigest(), "size": len(body), "mode": 0o600}
        metadata = json.dumps({"files": inventory}, sort_keys=True).encode()
        header = tarfile.TarInfo("recovery.json")
        header.size, header.mode = len(metadata), 0o600
        tar.addfile(header, io.BytesIO(metadata))
        if fail_finalization:
            raise OSError("injected finalization failure")
    writer.flush()
    if fail_fsync:
        raise OSError("injected fsync failure")
    # This is the publication boundary under test; errors above produce no pointer.
    return {"sha256": writer.hash.hexdigest(), "count": writer.count}


def main():
    cases = []
    files = {"state/empty": b"", "state/aligned": b"x" * 512,
             "state/unaligned": b"y" * 513, "assets/long/" + "z" * 110: bytes(range(256)) * 101,
             "state/unicode-α": b"\x00\xff\x00"}
    for name, output in [("normal", io.BytesIO()), ("short_writes", ShortWriter())]:
        pointer = archive(files, output)
        data = output.getvalue()
        assert pointer["sha256"] == hashlib.sha256(data).hexdigest()
        assert pointer["count"] == len(data) and len(data) % tarfile.RECORDSIZE == 0
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:") as tar:
            assert tar.getnames()[-1] == "recovery.json"
            metadata = json.load(tar.extractfile(tar.getmember("recovery.json")))
            assert set(tar.getnames()) == set(metadata["files"]) | {"recovery.json"}
            for member, expected in files.items():
                actual = tar.extractfile(member).read()
                assert actual == expected
                assert metadata["files"][member]["sha256"] == hashlib.sha256(actual).hexdigest()
        cases.append(name)
    for name, options in [("write_error", {"fail_at": 301}),
                          ("finalization_error", {"fail_finalization": True}),
                          ("fsync_error", {"fail_fsync": True})]:
        pointer = None
        try:
            pointer = archive(files, io.BytesIO(), **options)
        except OSError:
            pass
        assert pointer is None
        cases.append(name)
    try:
        with tarfile.open(fileobj=io.BytesIO(), mode="w") as tar:
            header = tarfile.TarInfo("short")
            header.size = 512
            tar.addfile(header, io.BytesIO(b"too short"))
    except OSError:
        cases.append("short_read_refused")
    else:
        raise AssertionError("short asset accepted")
    print(json.dumps({"outcome": "passed", "cases": cases,
                      "scope": "stdlib tar framing and hash I/O, not service backup/restore acceptance"}))


if __name__ == "__main__":
    main()
