"""Evidence integrity and unsafe extraction controls using the actual zstd/tar codecs."""

from __future__ import annotations

import io
import json
import tarfile
from compression import zstd
from pathlib import Path

import pytest

import storage_archive as archive


def test_roundtrip_preserves_bytes_modes_and_refuses_existing(tmp_path):
    source = tmp_path / "source"
    source.mkdir()
    (source / "raw").mkdir()
    (source / "raw" / "input.bin").write_bytes(bytes(range(256)) * 4096)
    (source / "raw" / "input.bin").chmod(0o640)
    result = archive.create_archive(source, tmp_path / "capture.tar.zst")
    assert result["outcome"] == "passed"
    assert not result["replay_qualified"]
    restored = tmp_path / "restored"
    assert archive.restore_archive(tmp_path / "capture.tar.zst", restored)["outcome"] == "passed"
    assert archive.inventory(source) == archive.inventory(restored)
    with pytest.raises(ValueError, match="already exists"):
        archive.restore_archive(tmp_path / "capture.tar.zst", restored)
    assert (source / "raw/input.bin").exists()


def test_compression_without_reader_qualification_cannot_replace_raw(tmp_path):
    source = tmp_path / "source"
    source.mkdir()
    (source / "capture").write_bytes(b"opaque")
    result = archive.create_archive(
        source, tmp_path / "capture.tar.zst", required_capabilities=("sampled-symbolized-report",)
    )
    assert result["outcome"] == "blocked"
    assert not (tmp_path / "capture.tar.zst").exists()
    assert (source / "capture").read_bytes() == b"opaque"


def malicious(path, name, kind=tarfile.REGTYPE, data=b"x", expected_sha=None):
    entry = {
        "path": name,
        "kind": "file",
        "mode": 0o600,
        "mtime_ns": 1,
        "size": len(data),
        "sha256": expected_sha or "invalid",
    }
    manifest = json.dumps({"schema": 1, "entries": [entry]}).encode()
    with zstd.open(path, "wb") as compressed, tarfile.open(fileobj=compressed, mode="w|") as tar:
        info = tarfile.TarInfo(archive.MANIFEST)
        info.size = len(manifest)
        tar.addfile(info, io.BytesIO(manifest))
        info = tarfile.TarInfo(name)
        info.type, info.size = kind, len(data) if kind == tarfile.REGTYPE else 0
        if kind == tarfile.SYMTYPE:
            info.linkname = "/etc/passwd"
        tar.addfile(info, io.BytesIO(data) if kind == tarfile.REGTYPE else None)


@pytest.mark.parametrize(
    "name,kind",
    [
        ("../escape", tarfile.REGTYPE),
        ("/absolute", tarfile.REGTYPE),
        ("link", tarfile.SYMTYPE),
        ("raw", tarfile.REGTYPE),
    ],
)
def test_unsafe_or_corrupt_archive_never_publishes(tmp_path, name, kind):
    path = tmp_path / "bad.tar.zst"
    malicious(path, name, kind)
    destination = tmp_path / "restored"
    with pytest.raises(ValueError):
        archive.restore_archive(path, destination)
    assert not destination.exists()
    assert not (tmp_path / "escape").exists()


def test_unknown_schema_and_truncated_frames_fail(tmp_path):
    source = tmp_path / "source"
    source.mkdir()
    (source / "raw").write_bytes(b"abcdef" * 10000)
    path = tmp_path / "capture.tar.zst"
    archive.create_archive(source, path)
    path.write_bytes(path.read_bytes()[:20])
    with pytest.raises((ValueError, EOFError, tarfile.TarError, zstd.ZstdError)):
        archive.restore_archive(path, tmp_path / "restored")
    assert (source / "raw").is_file()


def test_raw_symlinks_fail_closed(tmp_path):
    source = tmp_path / "source"
    source.mkdir()
    (source / "raw").symlink_to("/etc/passwd")
    with pytest.raises(ValueError, match="link"):
        archive.create_archive(source, tmp_path / "capture.tar.zst")


def test_unisolated_reader_success_does_not_qualify_archive(tmp_path):
    source = tmp_path / "source"
    source.mkdir()
    (source / "input").write_bytes(b"raw")
    result = archive.create_archive(
        source,
        tmp_path / "capture.tar.zst",
        replay=lambda _: {"capabilities": {"compiler-summary": {"outcome": "passed"}}},
        required_capabilities=("compiler-summary",),
    )
    assert result["outcome"] == "blocked"
    assert not (tmp_path / "capture.tar.zst").exists()
    assert (source / "input").read_bytes() == b"raw"


def test_same_filesystem_bind_mount_cannot_escape_archive_scope(tmp_path):
    import os
    import subprocess
    import sys

    source, foreign = tmp_path / "source", tmp_path / "foreign"
    source.mkdir()
    foreign.mkdir()
    (source / "nested").mkdir()
    (foreign / "private").write_text("foreign bytes")
    code = """import subprocess, sys
from pathlib import Path
import storage_archive
source, foreign = map(Path, sys.argv[1:])
subprocess.run(['mount', '--bind', str(foreign), str(source / 'nested')], check=True)
try:
    storage_archive.inventory(source)
except ValueError as error:
    assert 'filesystem boundary' in str(error)
else:
    raise AssertionError('same-filesystem bind mount was admitted')
"""
    result = subprocess.run(
        [
            "unshare",
            "--user",
            "--map-root-user",
            "--mount",
            "--fork",
            sys.executable,
            "-c",
            code,
            str(source),
            str(foreign),
        ],
        capture_output=True,
        text=True,
        env=dict(os.environ, PYTHONPATH=str(Path(archive.__file__).parent)),
    )
    if "Operation not permitted" in result.stderr:
        pytest.skip("blocked: disposable mount namespace unavailable")
    assert result.returncode == 0, result.stderr
    assert not list((source / "nested").iterdir())
    assert (foreign / "private").read_text() == "foreign bytes"
