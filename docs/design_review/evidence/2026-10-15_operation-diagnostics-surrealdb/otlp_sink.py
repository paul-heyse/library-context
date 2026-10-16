"""Trivial OTLP/gRPC sink for a bounded probe (stdlib only, Python 3.14).

Speaks just enough plaintext HTTP/2 (h2c prior knowledge) to accept OTLP
Export requests from opentelemetry-otlp's tonic exporter, answer them with an
empty successful gRPC response, and append a compact decoded summary per
request to an output JSONL file. One listening port per signal, so the signal
is known from the port and no HPACK decoding is needed.

Usage: otlp_sink.py OUT.jsonl TRACES_PORT METRICS_PORT LOGS_PORT
Not a collector: no TLS, no compression, no flow-control accounting beyond
returning every received byte as a window update.
"""

import json
import socket
import struct
import sys
import threading

PREFACE = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"
LOCK = threading.Lock()


def frame(ftype: int, flags: int, stream: int, payload: bytes = b"") -> bytes:
    return struct.pack(">I", len(payload))[1:] + bytes([ftype, flags]) + struct.pack(">I", stream) + payload


def lit(name: bytes, value: bytes) -> bytes:
    # HPACK literal header field without indexing, new name, no Huffman.
    return b"\x00" + bytes([len(name)]) + name + bytes([len(value)]) + value


def varint(buf: bytes, i: int) -> tuple[int, int]:
    shift = result = 0
    while True:
        b = buf[i]
        i += 1
        result |= (b & 0x7F) << shift
        if not b & 0x80:
            return result, i
        shift += 7


def fields(buf: bytes) -> list[tuple[int, int, object]]:
    out, i = [], 0
    while i < len(buf):
        key, i = varint(buf, i)
        num, wt = key >> 3, key & 7
        if wt == 0:
            v, i = varint(buf, i)
        elif wt == 1:
            v = buf[i : i + 8]
            i += 8
        elif wt == 2:
            n, i = varint(buf, i)
            v = buf[i : i + n]
            i += n
        elif wt == 5:
            v = buf[i : i + 4]
            i += 4
        else:
            raise ValueError(f"wire type {wt}")
        out.append((num, wt, v))
    return out


def sub(buf: bytes, num: int) -> list[bytes]:
    return [v for n, wt, v in fields(buf) if n == num and wt == 2]


def first(buf: bytes, num: int):
    for n, _wt, v in fields(buf):
        if n == num:
            return v
    return None


def anyvalue(buf: bytes):
    for n, wt, v in fields(buf):
        if n == 1:
            return v.decode("utf-8", "replace")
        if n == 2:
            return bool(v)
        if n == 3:
            return v
        if n == 4:
            return struct.unpack("<d", v)[0]
        return f"<field {n}>"
    return None


def attrs(buf: bytes, num: int) -> dict:
    out = {}
    for kv in sub(buf, num):
        k = first(kv, 1)
        v = first(kv, 2)
        out[k.decode() if k else "?"] = anyvalue(v) if v is not None else None
    return out


def decode_traces(msg: bytes) -> list[dict]:
    spans = []
    for rs in sub(msg, 1):
        res = first(rs, 1)
        res_attrs = attrs(res, 1) if res else {}
        for ss in sub(rs, 2):
            scope = first(ss, 1)
            scope_name = first(scope, 1).decode() if scope and first(scope, 1) else None
            for sp in sub(ss, 2):
                start = first(sp, 7)
                end = first(sp, 8)
                dur_ms = None
                if start and end:
                    dur_ms = (struct.unpack("<Q", end)[0] - struct.unpack("<Q", start)[0]) / 1e6
                parent = first(sp, 4)
                status = first(sp, 15)
                spans.append(
                    {
                        "name": (first(sp, 5) or b"").decode(),
                        "trace_id": (first(sp, 1) or b"").hex(),
                        "span_id": (first(sp, 2) or b"").hex(),
                        "parent_span_id": parent.hex() if parent else "",
                        "kind": first(sp, 6),
                        "duration_ms": dur_ms,
                        "attributes": attrs(sp, 9),
                        "events": [
                            {"name": (first(e, 2) or b"").decode(), "attributes": attrs(e, 3)}
                            for e in sub(sp, 11)
                        ],
                        "status": {
                            "code": first(status, 3),
                            "message": (first(status, 2) or b"").decode("utf-8", "replace"),
                        }
                        if status
                        else None,
                        "scope": scope_name,
                        "resource": res_attrs,
                    }
                )
    return spans


def decode_metrics(msg: bytes) -> list[dict]:
    out = []
    for rm in sub(msg, 1):
        for sm in sub(rm, 2):
            scope = first(sm, 1)
            scope_name = first(scope, 1).decode() if scope and first(scope, 1) else None
            for m in sub(sm, 2):
                name = (first(m, 1) or b"").decode()
                keys = set()
                points = 0
                for data_field, point_field, attr_field in ((5, 1, 7), (7, 1, 7), (9, 1, 9)):
                    for data in sub(m, data_field):
                        for dp in sub(data, point_field):
                            points += 1
                            keys.update(attrs(dp, attr_field).keys())
                out.append({"scope": scope_name, "name": name, "points": points, "attribute_keys": sorted(keys)})
    return out


def decode_logs(msg: bytes) -> list[dict]:
    out = []
    for rl in sub(msg, 1):
        for sl in sub(rl, 2):
            for lr in sub(sl, 2):
                body = first(lr, 5)
                out.append(
                    {
                        "severity_text": (first(lr, 3) or b"").decode(),
                        "body": anyvalue(body) if body is not None else None,
                        "attributes": attrs(lr, 6),
                        "trace_id": (first(lr, 9) or b"").hex(),
                    }
                )
    return out


DECODERS = {"traces": decode_traces, "metrics": decode_metrics, "logs": decode_logs}


def serve_conn(conn: socket.socket, signal: str, out_path: str) -> None:
    buf = b""

    def need(n: int) -> bytes:
        nonlocal buf
        while len(buf) < n:
            chunk = conn.recv(65536)
            if not chunk:
                raise EOFError
            buf += chunk
        data, buf = buf[:n], buf[n:]
        return data

    try:
        if need(len(PREFACE)) != PREFACE:
            return
        conn.sendall(frame(4, 0, 0))
        bodies: dict[int, bytes] = {}
        while True:
            hdr = need(9)
            length = int.from_bytes(hdr[:3], "big")
            ftype, flags = hdr[3], hdr[4]
            stream = int.from_bytes(hdr[5:9], "big") & 0x7FFFFFFF
            payload = need(length)
            if ftype == 4 and not flags & 1:
                conn.sendall(frame(4, 1, 0))
            elif ftype == 6 and not flags & 1:
                conn.sendall(frame(6, 1, 0, payload))
            elif ftype == 1:
                bodies.setdefault(stream, b"")
                if flags & 1:
                    bodies[stream] = bodies.get(stream, b"")
            elif ftype == 0:
                if flags & 0x8:
                    pad = payload[0]
                    payload = payload[1 : len(payload) - pad]
                bodies[stream] = bodies.get(stream, b"") + payload
                if length:
                    inc = struct.pack(">I", length)
                    conn.sendall(frame(8, 0, 0, inc) + frame(8, 0, stream, inc))
            if ftype in (0, 1) and flags & 1:
                body = bodies.pop(stream, b"")
                record = {"signal": signal, "bytes": len(body)}
                try:
                    compressed, mlen = body[0], int.from_bytes(body[1:5], "big")
                    record["grpc_compressed"] = compressed
                    if not compressed:
                        record["items"] = DECODERS[signal](body[5 : 5 + mlen])
                except Exception as exc:  # noqa: BLE001 - record and keep serving
                    record["decode_error"] = repr(exc)
                with LOCK, open(out_path, "a", encoding="utf-8") as fh:
                    fh.write(json.dumps(record, default=str) + "\n")
                headers = lit(b":status", b"200") + lit(b"content-type", b"application/grpc")
                trailers = lit(b"grpc-status", b"0")
                conn.sendall(
                    frame(1, 4, stream, headers)
                    + frame(0, 0, stream, b"\x00\x00\x00\x00\x00")
                    + frame(1, 5, stream, trailers)
                )
    except (EOFError, ConnectionError, OSError):
        pass
    finally:
        conn.close()


def listen(port: int, signal: str, out_path: str) -> None:
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", port))
    srv.listen()
    while True:
        conn, _ = srv.accept()
        threading.Thread(target=serve_conn, args=(conn, signal, out_path), daemon=True).start()


def main() -> None:
    out_path = sys.argv[1]
    ports = dict(zip(("traces", "metrics", "logs"), map(int, sys.argv[2:5]), strict=True))
    threads = [
        threading.Thread(target=listen, args=(p, s, out_path), daemon=True) for s, p in ports.items()
    ]
    for t in threads:
        t.start()
    print(f"otlp sink listening {ports}", flush=True)
    for t in threads:
        t.join()


if __name__ == "__main__":
    main()
