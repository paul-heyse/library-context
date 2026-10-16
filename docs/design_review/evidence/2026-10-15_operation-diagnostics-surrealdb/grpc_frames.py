"""Encode a SurrealDB gRPC QueryRequest frame / decode Query response frames.

stdlib only; used with `curl --http2-prior-knowledge` so no gRPC library is
needed. Field numbers come from surrealdb-protocol 0.13.1
(surrealdb.protocol.rpc.v1.QueryRequest: 2 = query; QueryResponse frames).

  grpc_frames.py encode 'RETURN 1' > req.bin
  grpc_frames.py decode resp.bin
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from otlp_sink import fields  # noqa: E402


def enc_varint(n: int) -> bytes:
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out.append(b | 0x80)
        else:
            out.append(b)
            return bytes(out)


def tree(buf: bytes, depth: int = 0):
    """Generic protobuf rendering: nested messages where they parse, else text/hex."""
    out = []
    for num, wt, v in fields(buf):
        if wt == 2:
            node = None
            if depth < 8:
                try:
                    sub = tree(v, depth + 1)
                    if sub:
                        node = sub
                except Exception:  # noqa: BLE001 - not a message, render as bytes
                    node = None
            if node is None:
                try:
                    node = v.decode("utf-8")
                except UnicodeDecodeError:
                    node = v.hex()
            out.append([num, node])
        elif wt in (1, 5):
            out.append([num, v.hex()])
        else:
            out.append([num, v])
    return out


def main() -> None:
    if sys.argv[1] == "encode":
        q = sys.argv[2].encode()
        msg = b"\x12" + enc_varint(len(q)) + q  # field 2, wire type 2
        sys.stdout.buffer.write(b"\x00" + len(msg).to_bytes(4, "big") + msg)
        return
    data = Path(sys.argv[2]).read_bytes()
    i = 0
    while i + 5 <= len(data):
        n = int.from_bytes(data[i + 1 : i + 5], "big")
        print(json.dumps(tree(data[i + 5 : i + 5 + n])))
        i += 5 + n


if __name__ == "__main__":
    main()
