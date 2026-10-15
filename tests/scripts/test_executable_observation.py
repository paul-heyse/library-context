"""Completion descriptors retain observations and refuse mutable-path races."""

import hashlib
import os

import pytest

import executable_observation as executable


def test_each_mutable_completion_hashes_and_later_replacement_preserves_receipt(
    tmp_path, monkeypatch
):
    path = tmp_path / "cargo-output"
    path.write_bytes(b"first executable")
    original = executable.hashlib.file_digest
    calls = []

    def hashed(stream, algorithm):
        calls.append(stream.fileno())
        return original(stream, algorithm)

    monkeypatch.setattr(executable.hashlib, "file_digest", hashed)
    first = executable.observe(path)
    again = executable.observe(path)
    assert len(calls) == 2
    assert first["sha256"] == again["sha256"] == hashlib.sha256(b"first executable").hexdigest()
    assert first["observed_at"] and first["descriptor"]["ino"] == path.stat().st_ino
    replacement = tmp_path / "replacement"
    replacement.write_bytes(b"later executable")
    os.replace(replacement, path)
    later = executable.observe(path)
    assert later["sha256"] != first["sha256"]
    assert first["sha256"] == hashlib.sha256(b"first executable").hexdigest()
    assert first["executed_bytes"] == first["source_closure"] == "unknown"
    assert first["policy"] == "completion-descriptor-sha256-v1"


@pytest.mark.parametrize("race", ["replace", "mutate", "unlink"])
def test_completion_refuses_descriptor_or_pathname_races(tmp_path, monkeypatch, race):
    path = tmp_path / "output"
    path.write_bytes(b"before")
    original = executable.hashlib.file_digest

    def raced(stream, algorithm):
        digest = original(stream, algorithm)
        if race == "replace":
            replacement = tmp_path / "replacement"
            replacement.write_bytes(b"after!")
            os.replace(replacement, path)
        elif race == "mutate":
            path.write_bytes(b"after!")
        else:
            path.unlink()
        return digest

    monkeypatch.setattr(executable.hashlib, "file_digest", raced)
    observed = executable.observe(path)
    assert observed["sha256"] is None
    assert observed["observation"] in ("changed", "unavailable")
    assert observed["descriptor"]["size"] == 6
    assert observed["observation_finished_at"]


def test_missing_completion_is_explicitly_unknown(tmp_path):
    observed = executable.observe(tmp_path / "missing")
    assert observed["observation"] == "unavailable" and observed["sha256"] is None
    assert observed["executed_bytes"] == observed["source_closure"] == "unknown"
