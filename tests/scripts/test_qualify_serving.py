"""Transcript retention includes malformed and final shutdown frames before decoding."""

import io
import queue

from qualify_serving import Stdio


def test_stdout_reader_retains_complete_lifetime_transcript(tmp_path):
    frames = [
        b"malformed protocol\n",
        b'{"jsonrpc":"2.0","id":1,"result":{}}\n',
        b'{"jsonrpc":"2.0","method":"notifications/message"}\n',
    ]
    reader = Stdio.__new__(Stdio)
    reader.stdout = io.BytesIO(b"".join(frames))
    reader.receipts = tmp_path
    reader.lines = queue.Queue()
    reader._read()
    assert (tmp_path / "protocol.jsonl").read_bytes() == b"".join(frames)
    assert [reader.lines.get_nowait() for _ in frames] == frames
    assert reader.lines.get_nowait() is None


def test_stdout_reader_redacts_urls_without_changing_delivered_failure_frame(tmp_path):
    raw = b'{"error":"postgresql://sensitive@127.0.0.1/example"}\n'
    reader = Stdio.__new__(Stdio)
    reader.stdout = io.BytesIO(raw)
    reader.receipts = tmp_path
    reader.lines = queue.Queue()
    reader._read()
    assert b"sensitive" not in (tmp_path / "protocol.jsonl").read_bytes()
    assert reader.lines.get_nowait() == raw
