"""Disposable real-owner, admission and interrupted-retirement controls."""

from __future__ import annotations

import datetime as dt
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

import storage_lifecycle as lifecycle


@pytest.fixture
def storage(tmp_path, monkeypatch):
    repo = tmp_path / "repository"
    (repo / ".config").mkdir(parents=True)
    (repo / ".config/storage.toml").write_text(
        'schema = 1\n[categories.scratch]\nowner = "task"\ngrace_days = 0\n'
        '[categories.unknown]\nowner = "external"\nprotected = true\n'
    )
    subprocess.run(["git", "init", "--quiet", str(repo)], check=True)
    monkeypatch.setenv("LCTX_STORAGE_CONFIG", str(tmp_path / "absent-host.toml"))
    monkeypatch.setenv("LCTX_STORAGE_STATE", str(tmp_path / "state"))
    monkeypatch.delenv(lifecycle.ADMISSION_ENV, raising=False)
    return lifecycle.Storage(root=repo, state=tmp_path / "state")


def publish(storage, path, *, parent=None, consumer="control", managed=True):
    path.mkdir(parents=True, exist_ok=True)
    (path / "output").write_text("owned disposable output")
    owner = {
        "kind": "task",
        "path": str(storage.root.parent / "removed-checkout"),
        "reference": "control-task-lifetime",
    }
    return storage.publish(path, "scratch", owner, consumer, managed=managed, parent=parent)


def release(storage, object_id, consumer="control"):
    storage.release(object_id, consumer, "owner released temporary output")


def child_environment():
    environment = dict(os.environ)
    environment["PYTHONPATH"] = str(Path(lifecycle.__file__).parent)
    return environment


@pytest.fixture
def processes():
    children = []

    def start(code, *args):
        process = subprocess.Popen(
            [sys.executable, "-c", code, *map(str, args)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=child_environment(),
        )
        children.append(process)
        return process

    yield start
    for process in children:
        if process.poll() is None:
            process.kill()
        process.communicate()


HOLD = """import sys
from pathlib import Path
from storage_lifecycle import admission
with admission([Path(sys.argv[1])], state=Path(sys.argv[2]), exclusive=sys.argv[3]=='exclusive'):
    print('ready', flush=True)
    sys.stdin.readline()
"""


def test_hold_release_unknown_and_actual_task_selection(storage):
    path = storage.root.parent / "scratch"
    object_id = publish(storage, path)
    assert storage.plan([object_id])["dispositions"][0]["disposition"] == "retained"
    release(storage, object_id)
    assert storage.plan([object_id])["dispositions"][0]["disposition"] == "eligible"
    storage.retain(object_id, "citation", "cited-report")
    with pytest.raises(lifecycle.Blocked, match="citation"):
        storage.retire(object_id)
    release(storage, object_id, "citation")
    checkout = storage.root.parent / "removed-checkout"
    (checkout / ".dev").mkdir(parents=True)
    (checkout / ".dev/build-dir.storage.json").write_text(
        json.dumps({"reference": "control-task-lifetime"})
    )
    assert storage.plan([object_id])["dispositions"][0]["disposition"] == "warm"
    (checkout / ".dev/build-dir.storage.json").unlink()
    (checkout / ".dev").rmdir()
    checkout.rmdir()
    assert storage.retire(object_id)["action"] == "retired"
    assert not path.exists()
    unknown = storage.root.parent / "unknown"
    unknown.mkdir()
    other = storage.publish(
        unknown, "unknown", {"kind": "external", "path": str(unknown)}, "external", managed=True
    )
    release(storage, other, "external")
    assert storage.plan([other])["dispositions"][0]["disposition"] == "external"
    with pytest.raises(lifecycle.Blocked):
        storage.retire(other)
    assert unknown.is_dir()


def test_explicit_grace_not_mtime_and_read_only_plan(storage):
    storage.config.write_text(
        storage.config.read_text().replace("grace_days = 0", "grace_days = 7")
    )
    storage = lifecycle.Storage(root=storage.root, state=storage.state)
    path = storage.root.parent / "scratch"
    object_id = publish(storage, path)
    release(storage, object_id)
    os.utime(path, (0, 0))
    before = {
        item: (item.read_bytes(), item.stat().st_mtime_ns)
        for item in storage.state.rglob("*")
        if item.is_file()
    }
    assert "grace" in " ".join(storage.plan([object_id])["dispositions"][0]["reasons"])
    row = storage.get(object_id)
    later = lifecycle.instant(row["obligations"]["control"]["released_at"]) + dt.timedelta(days=8)
    assert storage.disposition(row, references=[], clock=later)["disposition"] == "eligible"
    after = {
        item: (item.read_bytes(), item.stat().st_mtime_ns)
        for item in storage.state.rglob("*")
        if item.is_file()
    }
    assert after == before
    assert (path / "output").read_text() == "owned disposable output"


@pytest.mark.parametrize(
    "policy",
    [
        "schema = 99\n[categories]\n",
        'schema = 1\n[categories.scratch]\nowner = "invented"\n',
        'schema = 1\n[categories.scratch]\nowner = "task"\ngrace_days = -1\n',
    ],
)
def test_invalid_policy_does_not_prepare_or_remove(storage, policy):
    storage.config.write_text(policy)
    with pytest.raises(lifecycle.Invalid):
        lifecycle.Storage(root=storage.root, state=storage.state)
    assert not storage.state.exists()


def test_empty_plan_is_read_only(storage):
    assert storage.plan()["dispositions"] == []
    assert not storage.state.exists()


def test_real_shared_child_excludes_parent_retirement(storage, processes):
    parent = storage.root.parent / "outputs"
    child = parent / "child"
    child.mkdir(parents=True)
    process = processes(HOLD, child, storage.state, "shared")
    assert process.stdout.readline().strip() == "ready"
    with (
        pytest.raises(lifecycle.Blocked, match="busy"),
        lifecycle.admission([parent], exclusive=True, blocking=False, state=storage.state),
    ):
        pytest.fail("live child must protect the containing scope")
    process.communicate("done\n", timeout=15)
    assert process.returncode == 0
    with lifecycle.admission([parent], exclusive=True, blocking=False, state=storage.state):
        assert child.is_dir()


def test_real_parent_retirement_admission_excludes_new_child(storage, processes):
    parent = storage.root.parent / "outputs"
    parent.mkdir()
    child = parent / "new-child"
    code = """import sys, os
from pathlib import Path
from storage_lifecycle import admission, Blocked
os.environ.pop('LCTX_STORAGE_ADMISSION', None)
try:
    with admission([Path(sys.argv[1])], state=Path(sys.argv[2]), blocking=False):
        sys.exit(1)
except Blocked:
    print('blocked', flush=True)
sys.stdin.readline()
with admission([Path(sys.argv[1])], state=Path(sys.argv[2])):
    Path(sys.argv[1]).mkdir()
    print('created', flush=True)
"""
    with lifecycle.admission([parent], exclusive=True, state=storage.state):
        process = processes(code, child, storage.state)
        assert process.stdout.readline().strip() == "blocked"
        assert not child.exists()
    stdout, stderr = process.communicate("retry\n", timeout=15)
    assert process.returncode == 0, stderr
    assert stdout.strip() == "created"
    assert child.is_dir()


def test_non_lifo_shared_lifetimes_and_upgrade_refusal(storage, processes):
    first = storage.root.parent / "first"
    second = storage.root.parent / "second"
    outer = lifecycle.admission([first], state=storage.state)
    inner = lifecycle.admission([second], state=storage.state)
    outer.__enter__()
    inner.__enter__()
    try:
        outer.__exit__(None, None, None)
        with (
            pytest.raises(lifecycle.Blocked, match="upgrade"),
            lifecycle.admission([second], state=storage.state, exclusive=True),
        ):
            pytest.fail("shared ownership cannot upgrade")
        # A distinct process proves the surviving context still owns all its ancestors.
        code = """import sys, os
from pathlib import Path
from storage_lifecycle import admission, Blocked
os.environ.pop('LCTX_STORAGE_ADMISSION', None)
try:
    with admission([Path(sys.argv[1])], state=Path(sys.argv[2]), exclusive=True, blocking=False):
        sys.exit(1)
except Blocked:
    print('blocked')
"""
        process = processes(code, second, storage.state)
        stdout, stderr = process.communicate(timeout=15)
        assert process.returncode == 0, stderr
        assert stdout.strip() == "blocked"
    finally:
        inner.__exit__(None, None, None)
    assert lifecycle.ADMISSION_ENV not in os.environ


def test_actual_inherited_admission_reuses_and_refuses_upgrade(storage, processes):
    parent = storage.root.parent / "outputs"
    code = """import sys
from pathlib import Path
from storage_lifecycle import admission, Blocked
try:
    with admission([Path(sys.argv[1])], state=Path(sys.argv[2]),
                   exclusive=sys.argv[3]=='exclusive', blocking=False):
        print('admitted')
except Blocked as error:
    print('blocked:' + str(error))
"""
    with lifecycle.admission([parent], state=storage.state):
        process = processes(code, parent, storage.state, "shared")
        stdout, stderr = process.communicate(timeout=15)
        assert process.returncode == 0, stderr
        assert stdout.strip() == "admitted"
        process = processes(code, parent, storage.state, "exclusive")
        stdout, stderr = process.communicate(timeout=15)
        assert process.returncode == 0, stderr
        assert "inherited shared-to-exclusive" in stdout


def test_inherited_token_requires_actual_owned_fd(storage, processes, monkeypatch):
    parent = storage.root.parent / "outputs"
    code = """import sys
from pathlib import Path
from storage_lifecycle import admission, Blocked
try:
    with admission([Path(sys.argv[1])], state=Path(sys.argv[2])):
        sys.exit(1)
except Blocked:
    print('blocked')
"""
    with lifecycle.admission([parent], state=storage.state):
        rows = json.loads(os.environ[lifecycle.ADMISSION_ENV])
        descriptor = os.open(rows[-1]["key"], os.O_RDONLY)
        try:
            rows[-1]["fd"] = descriptor  # Same inode, but this FD owns no flock.
            with monkeypatch.context() as patch:
                patch.setenv(lifecycle.ADMISSION_ENV, json.dumps(rows))
                process = processes(code, parent, storage.state)
                stdout, stderr = process.communicate(timeout=15)
            assert process.returncode == 0, stderr
            assert stdout.strip() == "blocked"
        finally:
            os.close(descriptor)


def test_nonce_and_path_reuse_do_not_inherit_old_authority(storage):
    path = storage.root.parent / "scratch"
    object_id = publish(storage, path)
    release(storage, object_id)
    original_nonce = os.getxattr(path, lifecycle.NONCE_ATTRIBUTE)
    os.setxattr(path, lifecycle.NONCE_ATTRIBUTE, b"different-lifetime")
    assert "nonce" in " ".join(storage.plan([object_id])["dispositions"][0]["reasons"])
    os.setxattr(path, lifecycle.NONCE_ATTRIBUTE, original_nonce)
    original = path.with_name("original-output")
    path.rename(original)
    path.mkdir()
    (path / "output").write_text("new unrelated output")
    with pytest.raises(lifecycle.Blocked, match="identity"):
        storage.retire(object_id)
    assert (path / "output").read_text() == "new unrelated output"
    assert (original / "output").is_file()


def test_descendants_retire_independently(storage):
    parent = storage.root.parent / "outputs"
    parent_id = publish(storage, parent)
    child = parent / "child"
    child_id = publish(storage, child, parent=parent_id)
    release(storage, parent_id)
    release(storage, child_id)
    with pytest.raises(lifecycle.Blocked, match="descendant"):
        storage.retire(parent_id)
    assert storage.retire(child_id)["action"] == "retired"
    assert parent.is_dir()
    assert storage.retire(parent_id)["action"] == "retired"


def test_failure_before_rename_cancels_prepared_journal_then_retries(storage, monkeypatch):
    path = storage.root.parent / "scratch"
    object_id = publish(storage, path)
    release(storage, object_id)
    original = os.rename

    def fail(source, target, *args, **kwargs):
        if Path(source) == path:
            raise OSError("injected rename failure")
        return original(source, target, *args, **kwargs)

    with monkeypatch.context() as patch:
        patch.setattr(os, "rename", fail)
        with pytest.raises(OSError, match="injected"):
            storage.retire(object_id)
    assert path.is_dir()
    assert storage.get(object_id)["retirement"]
    assert storage.retire(object_id)["action"] == "prepared-retirement-cancelled"
    assert storage.retire(object_id)["action"] == "retired"
    assert storage.retire(object_id)["action"] == "already-retired"


def test_failure_after_rename_recovers_exact_quarantine_and_preserves_new_path(
    storage, monkeypatch
):
    path = storage.root.parent / "scratch"
    object_id = publish(storage, path)
    release(storage, object_id)
    original = lifecycle.fsync_directory

    def fail(directory):
        if directory == path.parent:
            raise OSError("injected parent fsync failure")
        return original(directory)

    with monkeypatch.context() as patch:
        patch.setattr(lifecycle, "fsync_directory", fail)
        with pytest.raises(OSError, match="injected"):
            storage.retire(object_id)
    assert not path.exists()
    assert (path.parent / (".lctx-retired-" + object_id)).is_dir()
    path.mkdir()
    (path / "new").write_text("new lifetime")
    assert storage.retire(object_id)["action"] == "retired"
    assert (path / "new").read_text() == "new lifetime"
    assert storage.retire(object_id)["action"] == "already-retired"


def test_deletion_failure_replays_and_leaf_symlink_never_follows(storage, monkeypatch):
    path = storage.root.parent / "scratch"
    object_id = publish(storage, path)
    release(storage, object_id)
    external = storage.root.parent / "external"
    external.mkdir()
    (external / "keep").write_text("protected external bytes")
    (path / "link").symlink_to(external, target_is_directory=True)
    with monkeypatch.context() as patch:
        patch.setattr(
            lifecycle,
            "_delete",
            lambda *args: (_ for _ in ()).throw(OSError("injected delete failure")),
        )
        with pytest.raises(OSError, match="injected"):
            storage.retire(object_id)
    assert storage.get(object_id)["retirement"]
    assert storage.retire(object_id)["action"] == "retired"
    assert (external / "keep").read_text() == "protected external bytes"


def test_stale_preview_cannot_override_new_hold_or_tracked_reference(storage):
    path = storage.root.parent / "scratch"
    object_id = publish(storage, path)
    release(storage, object_id)
    assert storage.plan([object_id])["dispositions"][0]["disposition"] == "eligible"
    storage.retain(object_id, "new-consumer", "raw-replay")
    with pytest.raises(lifecycle.Blocked, match="new-consumer"):
        storage.retire(object_id)
    release(storage, object_id, "new-consumer")
    citation = storage.root / "consumer.md"
    citation.write_text(f"Current consumer requires {path}\n")
    subprocess.run(["git", "-C", str(storage.root), "add", "consumer.md"], check=True)
    with pytest.raises(lifecycle.Blocked, match="tracked reference"):
        storage.retire(object_id)
    assert (path / "output").is_file()


@pytest.mark.parametrize(
    ("text", "cited"),
    [
        ("{id}", True),
        ("{path}", True),
        ("[exact](build/short)", True),
        ("[source relative](../build/short)", True),
        ("`build/short/output`", True),
        ("[encoded](<build/%73hort#details>)", True),
        ("[unrelated](build/other)", False),
        ("[undeclared ancestor](build)", False),
        ("[remote](https://example.org/build/short)", False),
        ("[remote](//example.org/build/short)", False),
        ("`build/short output`", False),
        ("[root](/) [dot](.) [parent](..)", False),
        ("__unreadable_participant__", True),
        ("{id} `http://[`", True),
        ("[first](build/short) `http://[`", True),
    ],
)
def test_prepared_reference_matching_preserves_branch_semantics(storage, text, cited):
    path = storage.root / "build/short"
    object_id = publish(storage, path)
    release(storage, object_id)
    source = storage.root / "docs/consumer.md"
    references = storage.prepare_references([(source, text.format(id=object_id, path=path))])
    decision = storage.disposition(storage.get(object_id), references=references)
    expected = (
        f"consumer coverage unavailable: {source}"
        if text.startswith("__") else f"current tracked reference: {source}"
    )
    assert decision["reasons"] == ([expected] if cited else [])
    assert decision["disposition"] == (
        "retained" if text.startswith("__") else "unresolved" if cited else "eligible"
    )


def test_prepared_references_preserve_name_matches_reason_order_and_duplicates(storage):
    path = storage.root / "long-component-name"
    object_id = publish(storage, path)
    release(storage, object_id)
    first, second = storage.root / "first.md", storage.root / "second.md"
    references = storage.prepare_references([
        (first, path.name),
        (second, "__missing_participant__"),
        (first, f"[one]({path}) [two]({path}/output)"),
    ])
    decision = storage.disposition(storage.get(object_id), references=references)
    assert decision["reasons"] == [
        f"current tracked reference: {first}",
        f"consumer coverage unavailable: {second}",
        f"current tracked reference: {first}",
    ]


@pytest.fixture(
    params=[("profile-raw", "compile-profile"), ("profile-report", "compile-profile-reports")]
)
def profile_scope(storage, request):
    from harness import ProcessIdentity, write_json_atomic

    category, component = request.param
    storage.config.write_text(
        storage.config.read_text() + f'\n[categories.{category}]\nowner="profile"\ngrace_days=0\n'
    )
    storage = lifecycle.Storage(root=storage.root, state=storage.state)
    run = storage.root / "runs/run01"
    run.mkdir(parents=True)
    (run / "owner.lock").touch()
    write_json_atomic(run / "record.json", {
        "schema": 2, "id": "run01", "owner": ProcessIdentity.of().to_json(),
        "child": None, "termination": "completed", "cleanup": {"status": "confirmed"},
        "ended": lifecycle.now(),
    })
    parent_id = publish(storage, run)
    release(storage, parent_id)
    path = run / component
    path.mkdir()
    (path / "output").write_text("retained profile content")
    object_id = storage.publish(
        path, category, {"kind": "profile", "path": str(run)}, "control", managed=True,
        parent=parent_id,
    )
    release(storage, object_id)
    return storage, object_id, path


@pytest.mark.parametrize(("text", "cited"), [
    ("Run just compile-profile record; inspect compile-profile-reports afterward.", False),
    ("`just compile-profile record --focus cpg-core`", False),
    ("Generic component `{component}` is used by the profiler.", False),
    ("Exact lifetime {id}", True),
    ("Exact output {path}", True),
    ("[exact](runs/run01/{component})", True),
    ("[output](runs/run01/{component}/output)", True),
    ("[declared run scope](runs/run01)", True),
])
def test_profile_tool_mentions_require_exact_lifetime_citation(profile_scope, text, cited):
    storage, object_id, path = profile_scope
    source = storage.root / "docs/consumer.md"
    text = text.format(id=object_id, path=path, component=path.name)
    decision = storage.disposition(storage.get(object_id), references=[(source, text)])
    assert decision["reasons"] == ([f"current tracked reference: {source}"] if cited else [])
    assert decision["disposition"] == ("unresolved" if cited else "eligible")
    assert (path / "output").read_text() == "retained profile content"


def test_profile_unique_basename_and_explicit_hold_remain_protective(profile_scope):
    storage, object_id, path = profile_scope
    unique = path.parent / "unique-profile-capture"
    unique.mkdir()
    row = storage.get(object_id)
    unique_id = storage.publish(
        unique, row["category"], row["owner"], "control", managed=True, parent=row["parent"]
    )
    release(storage, unique_id)
    source = storage.root / "consumer.md"
    decision = storage.disposition(storage.get(unique_id), references=[(source, unique.name)])
    assert decision["reasons"] == [f"current tracked reference: {source}"]
    storage.retain(object_id, "retained-evidence", "raw-replay")
    decision = storage.disposition(storage.get(object_id), references=[(source, path.name)])
    assert decision["reasons"] == ["consumer retained-evidence requires raw-replay"]
    assert decision["disposition"] == "retained"


@pytest.mark.parametrize("component", ["compile-profile", "compile-profile-reports"])
def test_nonprofile_canonical_basename_keeps_existing_alias_behavior(storage, component):
    path = storage.root / component
    object_id = publish(storage, path)
    release(storage, object_id)
    source = storage.root / "consumer.md"
    decision = storage.disposition(storage.get(object_id), references=[(source, component)])
    assert decision["reasons"] == [f"current tracked reference: {source}"]


def test_prepared_references_preserve_host_root_and_markdown_space_targets(storage):
    host = storage.root.parent / "host"
    path = host / "space name"
    object_id = publish(storage, path)
    release(storage, object_id)
    storage.host["repositories"] = [str(host)]
    source = storage.root / "docs/consumer.md"
    references = storage.prepare_references([(source, "[capture](<space%20name>)")])
    decision = storage.disposition(storage.get(object_id), references=references)
    assert decision["reasons"] == [f"current tracked reference: {source}"]


@pytest.mark.parametrize("changed_root", ["repository", "host"])
def test_prepared_references_rebind_different_resolution_roots(storage, changed_root):
    source = storage.root / "docs/consumer.md"
    references = storage.prepare_references([(source, "[capture](short)")])
    original_targets = list(references.targets(0))
    assert storage.prepare_references(references) is references
    other = lifecycle.Storage(
        root=storage.root.parent / "other" if changed_root == "repository" else storage.root,
        config=storage.config,
        state=storage.state.parent / "other-state",
    )
    if changed_root == "host":
        other.host["repositories"] = [str(storage.root.parent / "host")]
        path = storage.root.parent / "host/short"
    else:
        path = other.root / "short"
    object_id = publish(other, path)
    release(other, object_id)
    assert all(target != path for target, _ in original_targets)
    assert other.prepare_references(references) is not references
    decision = other.disposition(other.get(object_id), references=references)
    assert decision["reasons"] == [f"current tracked reference: {source}"]


def test_prepared_ancestor_citations_use_current_scope_status_before_effect(storage):
    parent = storage.root / "build"
    parent_id = publish(storage, parent)
    child = parent / "short"
    child_id = publish(storage, child, parent=parent_id)
    release(storage, parent_id)
    release(storage, child_id)
    source = storage.root / "consumer.md"
    references = storage.prepare_references([(source, "[scope](build)")])
    with pytest.raises(lifecycle.Blocked, match="tracked reference"):
        storage.retire(child_id, references=references)
    # The fixture simulates a current tombstone while keeping the physical child present.
    retired = storage.get(parent_id)
    retired["retired_at"] = lifecycle.now()
    storage.save(retired)
    assert storage.retire(child_id, references=references)["action"] == "retired"
    assert parent.is_dir()


def test_plan_prepares_document_tokens_once_but_not_across_operations(storage, monkeypatch):
    import urllib.parse

    ids = []
    for name in ("first", "second"):
        object_id = publish(storage, storage.root / "build" / name)
        release(storage, object_id)
        ids.append(object_id)
    source = storage.root / "consumer.md"
    source.write_text("[first](build/first/output) [second](build/second/output)")
    subprocess.run(["git", "-C", str(storage.root), "add", "consumer.md"], check=True)
    calls = {"tokens": 0, "urls": 0, "records": 0}
    findall, urlsplit, records = lifecycle.re.findall, urllib.parse.urlsplit, storage.records

    def counted_findall(*args):
        calls["tokens"] += 1
        return findall(*args)

    def counted_urlsplit(*args):
        calls["urls"] += 1
        return urlsplit(*args)

    def counted_records():
        calls["records"] += 1
        return records()

    monkeypatch.setattr(lifecycle.re, "findall", counted_findall)
    monkeypatch.setattr(urllib.parse, "urlsplit", counted_urlsplit)
    monkeypatch.setattr(storage, "records", counted_records)
    assert all(row["disposition"] == "unresolved" for row in storage.plan(ids)["dispositions"])
    assert calls == {"tokens": 1, "urls": 2, "records": 1}
    storage.plan(ids)
    assert calls == {"tokens": 2, "urls": 4, "records": 2}


def test_unmanaged_exposure_remains_protected_after_release(storage):
    path = storage.root.parent / "legacy"
    object_id = publish(storage, path, managed=False)
    release(storage, object_id)
    assert "unmanaged exposure" in " ".join(storage.plan([object_id])["dispositions"][0]["reasons"])
    with pytest.raises(lifecycle.Blocked, match="unmanaged exposure"):
        storage.retire(object_id)


def test_child_admission_survives_parent_scope_release(storage, processes):
    path = storage.root.parent / "owned-scope"
    path.mkdir()
    code = """import sys
from pathlib import Path
from storage_lifecycle import admission
path = Path(sys.argv[1])
with admission([path], state=Path(sys.argv[2])):
    print('ready', flush=True)
    sys.stdin.readline()
    with admission([path / 'nested'], state=Path(sys.argv[2])):
        print('usable', flush=True)
    sys.stdin.readline()
"""
    with lifecycle.admission([path], state=storage.state):
        process = processes(code, path, storage.state)
        assert process.stdout.readline().strip() == "ready"
    with (
        pytest.raises(lifecycle.Blocked, match="busy"),
        lifecycle.admission([path], exclusive=True, blocking=False, state=storage.state),
    ):
        pytest.fail("the child must own durable shared admission")
    process.stdin.write("parent released\n")
    process.stdin.flush()
    assert process.stdout.readline().strip() == "usable"
    stdout, stderr = process.communicate("done\n", timeout=15)
    assert process.returncode == 0, (stdout, stderr)
    with lifecycle.admission([path], exclusive=True, blocking=False, state=storage.state):
        assert path.is_dir()


def test_nested_exclusive_scope_cannot_reverse_lock_order(storage):
    outer = storage.root.parent / "z-last/deep"
    earlier = storage.root.parent / "a-first"
    with (
        lifecycle.admission([outer], exclusive=True, state=storage.state),
        pytest.raises(lifecycle.Blocked, match="reverse lock order"),
        lifecycle.admission([earlier], exclusive=True, state=storage.state),
    ):
        pytest.fail("a reverse exclusive acquisition must be refused")


def test_hardlinked_external_bytes_survive_retirement(storage):
    path = storage.root.parent / "scratch"
    object_id = publish(storage, path)
    release(storage, object_id)
    external = storage.root.parent / "externally-held-output"
    os.link(path / "output", external)
    assert storage.retire(object_id)["action"] == "retired"
    assert external.read_text() == "owned disposable output"


def test_quarantine_recovery_rebinds_run_owner_path(storage, monkeypatch):
    from harness import ProcessIdentity, write_json_atomic

    storage.config.write_text(
        storage.config.read_text() + '\n[categories.run-receipt]\nowner="run"\ngrace_days=0\n'
    )
    storage = lifecycle.Storage(root=storage.root, state=storage.state)
    path = storage.root.parent / "run-root"
    path.mkdir()
    (path / "owner.lock").touch()
    write_json_atomic(
        path / "record.json",
        {
            "schema": 2,
            "id": "control-run",
            "owner": ProcessIdentity.of().to_json(),
            "child": None,
            "termination": "completed",
            "cleanup": {"status": "confirmed"},
            "ended": lifecycle.now(),
        },
    )
    object_id = storage.publish(
        path, "run-receipt", {"kind": "run", "path": str(path)}, "control", managed=True
    )
    release(storage, object_id)
    with monkeypatch.context() as patch:
        patch.setattr(
            lifecycle,
            "_delete",
            lambda *args: (_ for _ in ()).throw(OSError("injected deletion interruption")),
        )
        with pytest.raises(OSError, match="injected"):
            storage.retire(object_id)
    assert not path.exists()
    assert storage.get(object_id)["retirement"]
    assert storage.retire(object_id)["action"] == "retired"
    assert storage.retire(object_id)["action"] == "already-retired"
