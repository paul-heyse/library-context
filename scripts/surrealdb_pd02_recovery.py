"""Explicit campaign attachment recovery, owned by service maintenance.

The immutable wire manifests enumerate policies, never executable SQL. A host
recovery receipt supplements the original unknown native owner receipt; it does
not rewrite it or infer remote finality from command/process cleanup.
Registered compiler owners, including failed terminal receipts and clean signal
owner siblings, use an exact-source shutdown proof and the existing installation
maintenance owner, without wire policy or request claims.
"""
from __future__ import annotations

import contextlib
import base64
import hashlib
import json
import os
import re
import secrets
import stat
import subprocess
import time
import uuid
from pathlib import Path

import surrealdb_service as service
from harness import ProcessIdentity, read_json, try_hold_lock
from storage_lifecycle import admission, durable_json, fsync_directory

OPERATION = "PD02 named attachment recovery"
NATIVE_OPERATION = "native named attachment recovery"
SOURCE_JOIN = {
    "kind": "source-correlated registered execution join",
    "upstream": "238bfeb11f5725bebed370167656748df8067595",
    "recipe": "635c2302b89a3a9995c3200ff79c373c37c619eddebbc6aee6a0d1ebd73b13f8",
    "patch": "1b93f5374afd6fc076cab76da465fe6eb61edcaa177402ed5d624284af9d284b",
    "success_path": "start serve Ok -> ntw shutdown_handler.await?? -> RPC registered drains join; CLI Err exits failure",
    "positive_shutdown_info": "not used as evidence by this distinct source-correlated proof",
}
POLICIES = {
    "attach_session": {}, "signin": {}, "use": {}, "detach_session": {},
    "define_table": {}, "session_read": {}, "table_rows": {}, "table_info": {},
    "remove_table": {}, "delivery_backpressure": {}, "no_output_window": {},
    "late_live_query": {},
    "normal_commit": {"normal": "present"},
    "before_poll": {"before_poll": "absent"},
    "inflight_transaction": {"inflight": "absent"},
    "committed_before_disconnect": {"committed": "present", "later": "absent"},
    "deadline": {"deadline": "absent"},
}
JOIN_MESSAGE = "Web server stopped. Bye!"
# Pinned upstream cleanup functions await but log these failures rather than
# returning them. A positive outer marker must not erase their unresolved state.
UNRESOLVED_MESSAGES = (
    "error cancelling grpc transaction", "error cleaning up live query",
    "error discarding live query", "transaction was dropped without being committed or cancelled",
    "error killing subscription-owned live query",
    "error cleaning up the live queries of an unfinished grpc streaming query",
    "grpc delivery-loss cleanup unresolved",
)


def journal_unresolved(row):
    message = row.get("MESSAGE", "")
    if not isinstance(message, str):
        return True
    if any(fragment in message.lower() for fragment in UNRESOLVED_MESSAGES):
        return True
    # stdout journald priority can remain INFO even for tracing's ERROR event.
    # Check both the native journal priority and the rendered tracing level.
    plain = re.sub(r"\x1b\[[0-9;]*m", "", message)
    priority = row.get("PRIORITY")
    return (priority is not None and str(priority) in {"0", "1", "2", "3"}
        or re.search(r"(?:^|\s)ERROR(?:\s|:)", plain) is not None)


def fail(detail):
    raise service.FixtureBlocked("named attachment recovery", detail,
        "resume the exact --recover-attachment ID or --recover-native-attachment ID operation")


def immutable(path: Path, value):
    """Publish once, fsync content and directory, then prohibit ordinary mutation."""
    data = json.dumps(value, sort_keys=True, indent=2).encode() + b"\n"
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "wb", closefd=False) as stream:
            stream.write(data)
            stream.flush()
            os.fsync(fd)
        os.fchmod(fd, 0o400)
        os.fsync(fd)
    finally:
        os.close(fd)
    fsync_directory(path.parent)


def checked_bytes(path: Path, *, readonly=False):
    if any(parent.is_symlink() for parent in (path, *path.parents)):
        fail("receipt path is not physical")
    info = path.stat()
    if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid()
            or stat.S_IMODE(info.st_mode) != (0o400 if readonly else 0o600)):
        fail("receipt is not a private owned regular file")
    return path.read_bytes()


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def manifests(directory, row):
    root = directory / "pd02-recovery"
    if not root.is_dir() or root.is_symlink():
        fail("wire owner did not declare immutable recovery manifests")
    result, sessions, tables, effects = {}, set(), set(), {}
    table_sessions, attach_tables, processes, sequences = {}, set(), set(), {}
    for path in sorted(root.iterdir()):
        if not re.fullmatch(r"[a-f0-9]{64}-[0-9]+\.json", path.name):
            fail("unexpected or partial wire manifest")
        data = checked_bytes(path, readonly=True)
        try:
            item = json.loads(data)
            table, sequence = item["table"], item["sequence"]
            policy = item["policy"]
            session = None if policy == "attach_session" else str(uuid.UUID(item["session"]))
            process = item["process"]
            if (set(process) != {"pid", "start_ticks"} or type(process["pid"]) is not int
                    or type(process["start_ticks"]) is not int or process["pid"] <= 0
                    or process["start_ticks"] < 0):
                fail("wire producer process identity differs")
            if (item.get("schema") != 1 or item.get("kind") != "PD02-wire-request"
                    or item.get("attachment") != row["id"]
                    or item.get("generation") != row["native_drain"]["generation"]
                    or item.get("namespace") != service.NAMESPACE
                    or item.get("database") != "validation"
                    or not re.fullmatch(r"pd02_[a-f0-9]{64}", table)
                    or type(sequence) is not int or sequence < 0
                    or path.name != f"{table[5:]}-{sequence}.json"
                    or session != item["session"] or policy not in POLICIES
                    or item.get("effects") != POLICIES[policy]
                    or item.get("live") != "absent_after_join"
                    or item.get("session_policy") != "volatile_named_session_after_join"
                    or not re.fullmatch(r"[a-f0-9]{64}", item["sql_blake3"])):
                fail("wire manifest identity/policy differs")
        except (KeyError, ValueError, TypeError):
            fail("wire manifest is malformed")
        if policy == "attach_session":
            attach_tables.add(table)
        else:
            sessions.add(session)
            if table_sessions.setdefault(table, session) != session:
                fail("table/request session chain differs")
        sequences.setdefault(table, {})[sequence] = policy
        processes.add((process["pid"], process["start_ticks"]))
        tables.add(table)
        result[path.name] = digest_bytes(data)
        for record, expected in POLICIES[policy].items():
            previous = effects.setdefault(table, {}).setdefault(record, expected)
            if previous != expected:
                fail("wire effects disagree")
    if not result or tables != attach_tables or tables != set(table_sessions):
        fail("wire owner allocation/session ownership chain is incomplete")
    for table, requests in sequences.items():
        if (sorted(requests) != list(range(len(requests)))
                or [requests.get(index) for index in range(3)] != ["attach_session", "signin", "use"]
                or any(policy in {"attach_session", "signin", "use"} for index, policy in requests.items() if index >= 3)):
            fail("wire request/session ownership prefix or sequence is incomplete")
    # Admission/session initialization may have failed before any table request;
    # absence then cannot establish table cleanup, so refuse instead of guessing.
    return {"files": result, "sessions": sorted(sessions), "tables": sorted(tables), "effects": effects, "processes": [list(process) for process in sorted(processes)]}


def native_hashes(directory):
    root = directory / "native-drain"
    if not root.is_dir() or root.is_symlink():
        fail("native owner directory is unavailable")
    result = {}
    for path in sorted(root.iterdir()):
        data = checked_bytes(path)
        row = json.loads(data)
        if (row.get("kind") != "native-process-owner" or row.get("owner") != "PD02 installed fixture wire owner"
                or row.get("schema") != 1):
            fail("unknown native owner is outside this bounded wire recovery")
        result[path.name] = digest_bytes(data)
    if not result:
        fail("no actual native owner receipt")
    return result


def inputs(directory, row):
    import surrealdb_fixture as fixture
    manifest = manifests(directory, row)
    native = fixture.native_completion(directory, row)
    files = native_hashes(directory)
    if len(native["owners"]) != len(files):
        fail("native owner binding is incomplete")
    processes = sorted({(owner["process"]["pid"], owner["process"]["start_ticks"])
        for owner in native["owners"]})
    if [list(process) for process in processes] != manifest["processes"]:
        fail("wire producer processes differ from actual native owners")
    return files, manifest


def native_owner_class(owner):
    """Finite owner classes; eligibility never promotes an original outcome."""
    store = owner["owner"] in {"native compiler store", "compiler workspace"}
    if (store
            and owner.get("state") == "started" and owner.get("completion") is None):
        return "Started native store/workspace owners"
    if (owner["owner"] == "native product cache session"
            and owner.get("state") == "started" and owner.get("completion") is None):
        # The registered cache session has volatile remote work, while persistent
        # products retain their separate quota/generation/lease lifecycle. Only the
        # checked daemon join establishes termination; this grants no cache deletion.
        return "Started native product cache session; original outcome unknown"
    completion = owner.get("completion")
    terminal = (owner.get("state") == "complete" and isinstance(completion, dict)
            and set(completion) == {"local", "remote", "storage", "committed", "failures"}
            and completion["local"] == "terminal" and completion["remote"] == "confirmed"
            and completion["storage"] == [] and completion["committed"] == []
            and isinstance(completion["failures"], list)
            and all(isinstance(item, dict) and set(item) == {"step", "error"}
                    and isinstance(item["step"], str) and item["step"]
                    and isinstance(item["error"], str) and item["error"]
                    for item in completion["failures"]))
    if terminal and store:
        if completion["failures"]:
            return "Failed terminal native store/workspace owners; original failure preserved"
        return "Clean terminal native store/workspace siblings; original outcome preserved"
    if (terminal and not completion["failures"] and owner["owner"] in {
            "native product cache session", "native terminal product source",
            "independent artifact admission"}):
        return f"Clean terminal {owner['owner']} sibling; original outcome preserved"
    if (terminal and owner["owner"] == "D10 direct native query signal owner"
            and not completion["failures"]):
        return "Clean terminal direct-query signal sibling; original outcome preserved"
    if (terminal and owner["owner"] == "S09 finite accepted-query wire owner"
            and completion["failures"]
            and all(item["step"] == "S09 actual wire control" for item in completion["failures"])):
        return "Failed terminal finite-query wire owner; original failure preserved"
    fail("native owner is outside the bounded registered store/workspace or terminal wire recovery")


def native_inputs(directory, row):
    """Bound receipts authorize no individual effect or attempt attribution."""
    import surrealdb_fixture as fixture
    native = fixture.native_completion(directory, row)
    if native.get("status") != "unknown" or native.get("reason") or not native.get("owners"):
        fail("native recovery requires bound actual eligible owners")
    original_owner = ProcessIdentity.from_json(row["owner"])
    if original_owner.foreign():
        fail("native owner namespace is unobservable")
    files = {}
    for owner in native["owners"]:
        native_owner_class(owner)
        process = ProcessIdentity.from_json({**original_owner.to_json(), **owner["process"]})
        if process.alive():
            fail("a declared native owner process is still live")
        path = Path(owner["path"])
        files[path.name] = digest_bytes(checked_bytes(path))
    return files, None


def validate_native_server(installation):
    import surrealdb_server
    if installation.record.get("kind") != "campaign":
        fail("native attachment recovery is campaign-only")
    if installation.record.get("server_log_filter", "warn") != "warn":
        fail("source-correlated native recovery is the explicit warn-only campaign route")
    generation = installation.record["binary"].get("generation", {})
    provenance = generation.get("provenance", {})
    selected = provenance.get("recipe", {})
    try:
        expected = surrealdb_server.recipe(cpu_count=selected["build"]["codegen_units"])
    except (KeyError, TypeError, ValueError):
        fail("exact reviewed server recipe is unavailable")
    if (selected != expected or surrealdb_server.identity(selected) != SOURCE_JOIN["recipe"]
            or provenance.get("recipe_identity") != SOURCE_JOIN["recipe"]
            or selected["pin"]["upstream_commit"] != SOURCE_JOIN["upstream"]
            or selected["source_patches"][0]["patch_sha256"] != SOURCE_JOIN["patch"]):
        fail("native recovery requires the exact reviewed registered-drain source recipe")
    service._installed_binary(installation)
    installer = Path(installation.record["installer"])
    if (installation.record.get("installer_generation", {}).get("path") != str(installer)
            or service.file_sha256(installer) != installation.record.get("installer_generation", {}).get("sha256")):
        fail("recorded native installer executable changed")
    expected_unit = service.unit_text(installation.directory, Path(installation.record["binary"]["path"]),
        int(installation.endpoint.rsplit(":", 1)[1]),
        log_filter=installation.record.get("server_log_filter", "warn"))
    if service._unit_path(installation.unit).read_text() != expected_unit:
        fail("native recovery unit differs from exact recorded binary/argv")


def validate_server(installation):
    if installation.record.get("kind") != "campaign":
        fail("named wire recovery is campaign-only")
    generation = installation.record["binary"].get("generation")
    if (not generation or generation["provenance"]["recipe"]["pin"]["build_metadata"]
            != "lctx.timeout-stack.grpc-drain.1"):
        fail("installed generation lacks the exact registered execution drain")
    service._installed_binary(installation)
    if installation.record.get("server_log_filter", "warn") != "warn,surrealdb::net=info":
        fail("positive registry-join logging has not been explicitly prepared")
    reference = installation.record.get("server_log_preparation")
    if not isinstance(reference, dict) or set(reference) != {"plan"}:
        fail("positive logging has no named preparation receipt")
    plan_path = Path(reference["plan"])
    if (plan_path.name != "plan.json" or plan_path.parent.parent != installation.directory / "service-log-preparations"
            or not re.fullmatch("[a-f0-9]{64}", plan_path.parent.name)):
        fail("logging preparation path differs")
    plan = json.loads(checked_bytes(plan_path, readonly=True))
    receipt = json.loads(checked_bytes(plan_path.with_name("receipt.json"), readonly=True))
    smoke = json.loads(checked_bytes(plan_path.with_name("empty-registry-plan.json"), readonly=True))
    if (plan.get("successor_descriptor") != installation.record
            or receipt.get("outcome") != "prepared"
            or receipt.get("plan_sha256") != service.file_sha256(plan_path)
            or receipt.get("binary") != installation.record["binary"]
            or receipt.get("empty_registry_plan_sha256") != service.file_sha256(plan_path.with_name("empty-registry-plan.json"))
            or receipt.get("empty_registry_join") != json.loads(checked_bytes(plan_path.with_name("empty-registry-joined.json"), readonly=True))
            or not valid_join_evidence(receipt["empty_registry_join"], smoke["daemon"], installation.unit)):
        fail("logging preparation receipt/descriptor differs")
    port = int(installation.endpoint.rsplit(":", 1)[1])
    expected = service.unit_text(installation.directory, Path(installation.record["binary"]["path"]),
        port, log_filter=installation.record["server_log_filter"])
    if service._unit_path(installation.unit).read_text() != expected:
        fail("owned unit differs from its recorded log filter/binary")


def journal_join(installation, invocation, pid=None, *, source_correlated=False):
    result = subprocess.run(["journalctl", "--user", "--no-pager", "--output=json",
        f"_SYSTEMD_INVOCATION_ID={invocation}", f"_SYSTEMD_USER_UNIT={installation.unit}"],
        capture_output=True, text=True, check=False, env=service.systemd_environment())
    if result.returncode:
        fail("exact invocation journal is unavailable")
    messages = []
    for line in result.stdout.splitlines():
        try:
            row = json.loads(line)
        except ValueError:
            fail("exact invocation journal is malformed")
        if row.get("_SYSTEMD_INVOCATION_ID") != invocation or row.get("_SYSTEMD_USER_UNIT") != installation.unit:
            fail("journal invocation/unit differs")
        messages.append(row)
    if any(journal_unresolved(row) for row in messages):
        fail("exact invocation contains unresolved transaction/LIVE cleanup evidence")
    positive = [row for row in messages if row.get("MESSAGE", "").endswith(JOIN_MESSAGE)
        and (pid is None or row.get("_PID") == str(pid))]
    if not source_correlated and len(positive) != 1:
        fail("missing unique positive post-registry-join shutdown message")
    if source_correlated and not any("SIGTERM received. Waiting for graceful shutdown;" in row.get("MESSAGE", "")
            and row.get("_PID") == str(pid) for row in messages):
        fail("exact daemon did not journal the deliberately delivered SIGTERM")
    # The checked source propagates unresolved registry/session finalizers rather
    # than reaching its positive message. Retain the whole exact journal as evidence.
    return {"sha256": digest_bytes(result.stdout.encode()), "text": result.stdout, "entries": messages}


def valid_join_evidence(joined, daemon, unit, *, source_correlated=False):
    try:
        state, journal = joined["state"], joined["journal"]
        entries = [json.loads(line) for line in journal["text"].splitlines()]
        if (journal["sha256"] != digest_bytes(journal["text"].encode())
                or journal["entries"] != entries
                or any(journal_unresolved(entry) for entry in entries)
                or state.get("InvocationID") != daemon["invocation"]
                or state.get("MainPID") != "0" or state.get("ActiveState") != "inactive"
                or state.get("Result") != "success" or state.get("ExecMainCode") != "1"
                or state.get("ExecMainStatus") != "0"
                or any(entry.get("_SYSTEMD_INVOCATION_ID") != daemon["invocation"]
                    or entry.get("_SYSTEMD_USER_UNIT") != unit for entry in entries)):
            return False
        if source_correlated:
            return joined.get("source_proof") == SOURCE_JOIN and any(
                "SIGTERM received. Waiting for graceful shutdown;" in entry.get("MESSAGE", "")
                and entry.get("_PID") == str(daemon["process"]["pid"]) for entry in entries)
        return len([entry for entry in entries if entry.get("MESSAGE", "").endswith(JOIN_MESSAGE)
            and entry.get("_PID") == str(daemon["process"]["pid"])]) == 1
    except (ValueError, TypeError, KeyError, AttributeError):
        return False


def joined_exit(installation, plan, *, source_correlated=False):
    old = plan["daemon"]
    state = service.unit_properties(installation.unit, "InvocationID", "MainPID", "ActiveState",
        "Result", "ExecMainCode", "ExecMainStatus")
    if state.get("InvocationID") != old["invocation"]:
        fail("server invocation changed before its checked registry join")
    if state.get("MainPID") not in ("0", str(old["process"]["pid"])):
        fail("server PID changed before its checked registry join")
    process = ProcessIdentity.from_json(old["process"])
    if state.get("ActiveState") == "active":
        if not process.alive():
            fail("old daemon process identity is not live")
        if service.systemctl("kill", "--signal=SIGTERM", "--kill-whom=main", installation.unit).returncode:
            fail("cannot signal the exact daemon for cooperative registry join")
    # No stop job, timeout or forced signal: interruption retains the durable
    # maintenance marker and original invocation for a later explicit resume.
    while True:
        state = service.unit_properties(installation.unit, "InvocationID", "MainPID", "ActiveState",
            "Result", "ExecMainCode", "ExecMainStatus")
        if state.get("InvocationID") != old["invocation"]:
            fail("server invocation changed while joining")
        if state.get("ActiveState") == "inactive" and state.get("MainPID") == "0":
            break
        if state.get("ActiveState") == "failed":
            fail("server failed during registry join; unknown remains recovery-owned")
        time.sleep(0.2)
    if (state.get("Result") != "success" or state.get("ExecMainCode") != "1"
            or state.get("ExecMainStatus") != "0" or process.alive()):
        fail("server exit is not the exact invocation's clean exited/0")
    journal = (journal_join(installation, old["invocation"], old["process"]["pid"], source_correlated=True)
        if source_correlated else journal_join(installation, old["invocation"], old["process"]["pid"]))
    result = {"state": state, "journal": journal}
    if source_correlated:
        result["source_proof"] = SOURCE_JOIN
    return result


def reconcile(installation, manifest, new_process):
    cfg = installation.runtime("validation", installer=True)
    def value(statement):
        installation.check_daemon()
        rows = service.sql(installation, statement, cfg=cfg)
        if len(rows) != 1 or not isinstance(rows[0], dict) or rows[0].get("status") != "OK":
            fail("reconciliation did not return one confirmed terminal result")
        return rows[0].get("result")
    outcomes = {}
    for table in manifest["tables"]:
        info = value(f"INFO FOR TABLE {table}")
        if not isinstance(info, dict) or info.get("lives") != {}:
            fail("exact owned table LIVE inventory is not empty")
        effects = {}
        for record, expected in manifest["effects"].get(table, {}).items():
            exists = value(f"RETURN record::exists({table}:{record})")
            if type(exists) is not bool or exists != (expected == "present"):
                fail(f"exact owned effect {table}:{record} disagrees with {expected} policy")
            effects[record] = expected
        records = value(f"SELECT VALUE record::id(id) FROM {table}")
        expected = sorted(record for record, state in effects.items() if state == "present")
        if not isinstance(records, list) or sorted(records) != expected:
            fail("exact owned table contains undeclared/missing rows")
        outcomes[table] = {"live": "absent", "effects": effects, "records": records}
    # Verify all tables/effects before deleting any. A later interruption is
    # recoverable only from the recorded per-table terminal cleanup evidence.
    return outcomes


def recovered(directory, record):
    """Read-only consumer: original unknown remains visible, host receipt is separate."""
    try:
        reference = (record or {}).get("host_recovery")
        if not isinstance(reference, dict):
            return False
        if reference.get("kind") == NATIVE_OPERATION:
            return native_recovered(directory, record)
        root = directory / "pd02-host-recovery"
        receipt_path, plan_path = root / "receipt.json", root / "plan.json"
        data = checked_bytes(receipt_path, readonly=True)
        plan_data = checked_bytes(plan_path, readonly=True)
        receipt, plan = json.loads(data), json.loads(plan_data)
        if (reference != {"path": str(receipt_path), "sha256": digest_bytes(data)}
                or receipt.get("kind") != OPERATION or receipt.get("outcome") != "recovered"
                or receipt.get("plan_sha256") != digest_bytes(plan_data)
                or plan.get("attachment") != record["id"]
                or plan.get("fixture") != record["fixture"]
                or plan.get("generation") != record["native_drain"]["generation"]
                or inputs(directory, record) != (plan["native_files"], plan["manifests"])
                or digest_bytes(base64.b64decode(plan["original_record_bytes"], validate=True)) != plan["original_record_sha256"]
                or json.loads(base64.b64decode(plan["original_record_bytes"], validate=True)) != plan["original_record"]
                or json.loads(checked_bytes(root / "joined.json", readonly=True)) != receipt["joined"]
                or json.loads(checked_bytes(root / "reconciliation.json", readonly=True)) != receipt["reconciliation"]
                or digest_bytes(base64.b64decode(plan["descriptor_bytes"], validate=True)) != plan["descriptor_sha256"]
                or json.loads(base64.b64decode(plan["descriptor_bytes"], validate=True)) != plan["descriptor"]
                or plan["descriptor"]["installation_id"] != plan["fixture"]
                or bytes(plan["descriptor"]["service_generation"]).hex() != plan["generation"]
                or plan["original_record"]["native_drain"] != record["native_drain"]
                or receipt.get("native_drain") != "confirmed"
                or set(receipt["tables_removed"]) != set(plan["manifests"]["tables"])
                or set(receipt["reconciliation"]) != set(plan["manifests"]["tables"])
                or receipt["new_daemon"] == plan["daemon"]["process"]
                or receipt["session_premise"] != "pinned volatile RpcState; exact joined invocation replaced"
                or not valid_join_evidence(receipt["joined"], plan["daemon"], service.installation_unit(plan["descriptor"]))):
            return False
        for table in plan["manifests"]["tables"]:
            outcome = receipt["reconciliation"][table]
            effects = plan["manifests"]["effects"].get(table, {})
            terminal_path = root / f"removed-{table}.json"
            terminal_bytes = checked_bytes(terminal_path, readonly=True)
            terminal = json.loads(terminal_bytes)
            if (outcome != {"live": "absent", "effects": effects,
                    "records": sorted(key for key, state in effects.items() if state == "present")}
                    or receipt["cleanup_files"].get(terminal_path.name) != digest_bytes(terminal_bytes)
                    or terminal.get("table") != table or not isinstance(terminal.get("terminal"), list)
                    or len(terminal["terminal"]) != 1 or terminal["terminal"][0].get("status") != "OK"):
                return False
        return True
    except (OSError, ValueError, KeyError, TypeError, AttributeError, service.FixtureBlocked):
        return False


def native_scope_snapshot(directory, record):
    """Exact named local owners; historical release confers no transport finality."""
    import surrealdb_fixture as fixture
    if record.get("native_drain") is not None:
        files, _ = native_inputs(directory, record)
        classes = {native_owner_class(owner) for owner in fixture.native_completion(directory, record)["owners"]}
        classification = "; ".join(sorted(classes))
    else:
        native_root = directory / "native-drain"
        if (record.get("released") is not True or record.get("native_completion") is not None
                or native_root.exists() and (native_root.is_symlink() or not native_root.is_dir() or any(native_root.iterdir()))):
            fail("legacy peer lacks its historical local release or has native observations")
        files, classification = {}, "historical local release; legacy native outcome/finality not retrospectively qualified"
    if ((record.get("command_cleanup") or {}).get("status") != "confirmed"
            or (record.get("command_cleanup") or {}).get("survivors")
            or fixture._command_observation(record)["status"] != "gone"):
        fail("named peer command cleanup is not confirmed")
    data = checked_bytes(directory / "record.json")
    return {"classification": classification, "original_record": record,
        "original_bytes": base64.b64encode(data).decode(), "original_sha256": digest_bytes(data),
        "native_files": files, "native_bytes": {name: base64.b64encode(checked_bytes(directory / "native-drain" / name)).decode()
            for name in files}}


def same_original(original, current):
    mutable = {"host_recovery", "released", "released_at"}
    return {key: value for key, value in original.items() if key not in mutable} == {
        key: value for key, value in current.items() if key not in mutable}


def valid_native_scope(directory, plan):
    try:
        transferred = plan.get("transferred_maintenance")
        if transferred is not None:
            raw = base64.b64decode(transferred["bytes"], validate=True)
            prior = transferred["record"]
            if (set(transferred) != {"record", "bytes", "sha256"}
                    or digest_bytes(raw) != transferred["sha256"] or json.loads(raw) != prior
                    or set(prior) != {"operation", "token", "owner"}
                    or prior["operation"] != "maintenance"
                    or not isinstance(prior["token"], str) or not prior["token"]):
                return False
            ProcessIdentity.from_json(prior["owner"])
        scope = plan["named_scope"]
        if not scope or plan["attachment"] not in scope:
            return False
        for identity, snapshot in scope.items():
            if str(uuid.UUID(identity)) != identity:
                return False
            peer = directory.parent / identity
            current = read_json(peer / "record.json")
            original = snapshot["original_record"]
            raw = base64.b64decode(snapshot["original_bytes"], validate=True)
            if (digest_bytes(raw) != snapshot["original_sha256"] or json.loads(raw) != original
                    or not same_original(original, current) or original["id"] != identity
                    or original["fixture"] != plan["fixture"]):
                return False
            checked = native_scope_snapshot(peer, current)
            if (checked["classification"] != snapshot["classification"]
                    or checked["native_files"] != snapshot["native_files"]
                    or checked["native_bytes"] != snapshot["native_bytes"]):
                return False
        legacy = {identity for identity,snapshot in scope.items() if snapshot["classification"].startswith("historical")}
        if set(plan["legacy_runs"]) != legacy:
            return False
        for identity,snapshot in plan["legacy_runs"].items():
            raw=base64.b64decode(snapshot["record_bytes"],validate=True)
            run=json.loads(raw)
            original=scope[identity]["original_record"]
            if (digest_bytes(raw) != snapshot["record_sha256"] or run != snapshot["record"]
                    or run.get("phase") != "ended" or run.get("cleanup",{}).get("status") != "confirmed"
                    or run.get("cleanup",{}).get("survivors")
                    or run.get("started") != original["created"] or run.get("ended") != original["released_at"]
                    or run.get("cwd") != original["checkout"]
                    or set(snapshot["logs"]) != set(run["logs"])
                    or any(value["path"] != run["logs"][name] for name,value in snapshot["logs"].items())
                    or snapshot.get("evidence_scope") != "original run preservation/correspondence only; no prerequest or native finality claim"
                    or any(digest_bytes(base64.b64decode(value["bytes"],validate=True)) != value["sha256"] for value in snapshot["logs"].values())):
                return False
        return True
    except (OSError, ValueError, KeyError, TypeError, AttributeError, service.FixtureBlocked):
        return False


def native_recovered(directory, record):
    try:
        reference = record["host_recovery"]
        primary = reference["primary"]
        if str(uuid.UUID(primary)) != primary or reference.get("kind") != NATIVE_OPERATION:
            return False
        primary_directory = directory.parent / primary
        primary_record = read_json(primary_directory / "record.json")
        root = primary_directory / "native-host-recovery"
        data = checked_bytes(root / "receipt.json", readonly=True)
        plan = json.loads(checked_bytes(root / "plan.json", readonly=True))
        expected = {"kind": NATIVE_OPERATION, "primary": primary, "path": str(root / "receipt.json"), "sha256": digest_bytes(data)}
        return (reference == expected and primary_record.get("host_recovery") == expected
            and directory.name in plan["named_scope"] and same_original(plan["named_scope"][directory.name]["original_record"],record)
            and valid_native_scope(primary_directory, plan)
            and _native_primary_recovered(primary_directory, primary_record))
    except (OSError, ValueError, KeyError, TypeError, AttributeError, service.FixtureBlocked):
        return False


def _native_primary_recovered(directory, record):
    """A source-correlated host recovery never promotes individual outcomes."""
    try:
        root = directory / "native-host-recovery"
        data = checked_bytes(root / "receipt.json", readonly=True)
        plan_data = checked_bytes(root / "plan.json", readonly=True)
        receipt, plan = json.loads(data), json.loads(plan_data)
        original_bytes = base64.b64decode(plan["original_record_bytes"], validate=True)
        descriptor_bytes = base64.b64decode(plan["descriptor_bytes"], validate=True)
        files, manifest = native_inputs(directory, record)
        original = plan["original_record"]
        mutable = {"host_recovery", "released", "released_at"}
        argv = base64.b64decode(plan["daemon"]["argv"], validate=True).split(b"\0")
        descriptor = plan["descriptor"]
        old_process = ProcessIdentity.from_json(plan["daemon"]["process"])
        new_process = ProcessIdentity.from_json(receipt["new_daemon"])
        if ((record or {}).get("host_recovery") != {"kind":NATIVE_OPERATION,"primary":directory.name,"path": str(root / "receipt.json"), "sha256": digest_bytes(data)}
                or receipt.get("kind") != NATIVE_OPERATION or plan.get("kind") != NATIVE_OPERATION
                or receipt.get("outcome") != "recovered" or receipt.get("plan_sha256") != digest_bytes(plan_data)
                or plan.get("source_proof") != SOURCE_JOIN or receipt.get("source_proof") != SOURCE_JOIN
                or plan.get("attachment") != record["id"] or plan.get("fixture") != record["fixture"]
                or plan.get("generation") != record["native_drain"]["generation"]
                or plan.get("native_files") != files or plan.get("manifests") is not None or manifest is not None
                or set(plan["native_bytes"]) != set(files)
                or any(digest_bytes(base64.b64decode(value, validate=True)) != files[name]
                    for name, value in plan["native_bytes"].items())
                or digest_bytes(original_bytes) != plan["original_record_sha256"]
                or json.loads(original_bytes) != plan["original_record"]
                or {key: value for key, value in original.items() if key not in mutable}
                    != {key: value for key, value in record.items() if key not in mutable}
                or plan["original_record"]["native_drain"] != record["native_drain"]
                or plan["original_record"]["owner"] != record["owner"]
                or plan["original_record"]["command_cleanup"] != record["command_cleanup"]
                or digest_bytes(descriptor_bytes) != plan["descriptor_sha256"]
                or json.loads(descriptor_bytes) != plan["descriptor"]
                or plan["descriptor"].get("kind") != "campaign"
                or plan["descriptor"]["installation_id"] != plan["fixture"]
                or bytes(plan["descriptor"]["service_generation"]).hex() != plan["generation"]
                or descriptor.get("server_log_filter", "warn") != "warn"
                or argv[0] != descriptor["binary"]["path"].encode() or b"start" not in argv
                or b"--log=warn" not in argv
                or descriptor["endpoint"].removeprefix("http://").encode() not in argv
                or ("rocksdb://" + str(Path(descriptor["state_root"]) / "data/store")).encode() not in argv
                or not isinstance(plan["daemon"]["executable_identity"], list)
                or len(plan["daemon"]["executable_identity"]) != 5
                or receipt.get("new_daemon") == plan["daemon"]["process"]
                or new_process.boot_id != old_process.boot_id or new_process.pid_namespace != old_process.pid_namespace
                or not re.fullmatch("[a-f0-9]{32}", receipt.get("new_invocation", ""))
                or receipt.get("new_invocation") == plan["daemon"]["invocation"]
                or receipt.get("native_drain") != "confirmed"
                or receipt.get("recovery_scope") != "installation-wide persisted-owner recovery under exclusive maintenance"
                or receipt.get("original_request_outcomes") != "unknown; no original request/attempt attribution available"
                or receipt.get("maintenance_actions") != ["close-admission", "drain", "check"]
                or receipt.get("named_scope") != {identity: snapshot["classification"] for identity,snapshot in plan["named_scope"].items()}
                or receipt.get("reconciliation") is not None or receipt.get("tables_removed") != []
                or receipt.get("cleanup_files") != {}
                or json.loads(checked_bytes(root / "joined.json", readonly=True)) != receipt["joined"]
                or not valid_join_evidence(receipt["joined"], plan["daemon"], service.installation_unit(plan["descriptor"]),
                    source_correlated=True)):
            return False
        import surrealdb_server
        selected = plan["descriptor"]["binary"]["generation"]["provenance"]["recipe"]
        return (surrealdb_server.identity(selected) == SOURCE_JOIN["recipe"]
            and selected["pin"]["upstream_commit"] == SOURCE_JOIN["upstream"]
            and selected["source_patches"][0]["patch_sha256"] == SOURCE_JOIN["patch"])
    except (OSError, ValueError, KeyError, IndexError, TypeError, AttributeError, service.FixtureBlocked):
        return False


def native_daemon_snapshot(installation, process):
    argv = Path(f"/proc/{process.pid}/cmdline").read_bytes()
    executable = service.executable_identity(Path(f"/proc/{process.pid}/exe").stat())
    if not process.alive() or executable != installation._daemon_identity[1]:
        fail("checked native recovery daemon executable changed")
    return {"argv": base64.b64encode(argv).decode(), "executable_identity": list(executable)}


def legacy_run_snapshot(run_id, original):
    import runs
    if not re.fullmatch(r"[0-9]{8}T[0-9]{6}\.[0-9]{3}Z-[a-f0-9]{6}",run_id):
        fail("legacy peer requires an exact retained run ID")
    directory=runs.runs_root() / run_id
    raw=checked_bytes(directory / "record.json")
    run=json.loads(raw)
    if (run.get("id") != run_id or run.get("phase") != "ended"
            or run.get("cleanup",{}).get("status") != "confirmed" or run.get("cleanup",{}).get("survivors")
            or run.get("started") != original["created"] or run.get("ended") != original["released_at"]
            or run.get("cwd") != original["checkout"]):
        fail("legacy original run correspondence/cleanup differs")
    logs={}
    for name,value in run["logs"].items():
        path=Path(value)
        if path.parent != directory or any(parent.is_symlink() for parent in (path,*path.parents)):
            fail("legacy run log is outside its physical exact run")
        info=path.stat()
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid():
            fail("legacy run log is not owned regular evidence")
        data=path.read_bytes()
        logs[name]={"path":str(path),"bytes":base64.b64encode(data).decode(),"sha256":digest_bytes(data)}
    return {"record":run,"record_bytes":base64.b64encode(raw).decode(),"record_sha256":digest_bytes(raw),"logs":logs,
        "evidence_scope":"original run preservation/correspondence only; no prerequest or native finality claim"}


def recover_native(installation, attachment_ids, *, legacy_runs=None):
    if not attachment_ids or len(set(attachment_ids)) != len(attachment_ids):
        fail("native recovery requires distinct explicitly named attachments")
    for identity in attachment_ids:
        if str(uuid.UUID(identity)) != identity:
            fail("invalid exact attachment ID")
    started = [identity for identity in attachment_ids
        if (read_json(installation.directory / "attachments" / identity / "record.json") or {}).get("native_drain") is not None]
    if not started:
        fail("joint recovery requires an actual eligible native owner")
    primary = sorted(started)[0]
    return recover(installation,primary,native=True,peers=sorted(set(attachment_ids)-{primary}),legacy_runs=legacy_runs or {})


def recover(installation, attachment_id, *, native=False, peers=(), legacy_runs=None):
    import surrealdb_fixture as fixture
    operation = NATIVE_OPERATION if native else OPERATION
    try:
        if str(uuid.UUID(attachment_id)) != attachment_id:
            fail("invalid exact attachment ID")
    except ValueError:
        fail("invalid exact attachment ID")
    directory = installation.directory / "attachments" / attachment_id
    marker_path = installation.directory / "maintenance.json"
    with admission([installation.directory, directory]):
        gate = try_hold_lock(installation.directory / "admission.lock")
        if gate is None:
            fail("admission maintenance owner is live")
        owner_lock = use = None
        maintenance_claimed = False
        peer_locks, peer_rows = {}, {}
        try:
            marker = read_json(marker_path)
            transferred_maintenance = None
            if marker and native and marker.get("operation") == "maintenance":
                if set(marker) != {"operation", "token", "owner"} or not isinstance(marker["token"], str) or not marker["token"]:
                    fail("ordinary maintenance marker carries additional or malformed ownership")
                prior_owner = ProcessIdentity.from_json(marker["owner"])
                if prior_owner.foreign() or prior_owner.alive():
                    fail("ordinary maintenance owner is live or unobservable")
                raw = checked_bytes(marker_path)
                if json.loads(raw) != marker:
                    fail("ordinary maintenance marker changed during transfer")
                transferred_maintenance = {"record": marker, "bytes": base64.b64encode(raw).decode(), "sha256": digest_bytes(raw)}
                marker = None
            if marker and (marker.get("operation") != operation or marker.get("attachment") != attachment_id
                    or ProcessIdentity.from_json(marker["owner"]).alive()):
                fail("another or live maintenance owner must be resolved first")
            row = read_json(directory / "record.json")
            if not row or row.get("fixture") != installation.id or row.get("id") != attachment_id:
                fail("attachment does not belong to this installation")
            try:
                local_owner = ProcessIdentity.from_json(row["owner"])
            except (KeyError, TypeError, ValueError):
                fail("attachment owner identity is unavailable")
            if local_owner.foreign():
                fail("attachment owner belongs to an unobservable PID namespace")
            if fixture.owner_alive(row.get("owner"), directory / "owner.lock"):
                fail("attachment owner is live")
            owner_lock = try_hold_lock(directory / "owner.lock")
            if owner_lock is None or read_json(directory / "record.json") != row:
                fail("attachment owner/receipt changed")
            for identity in peers:
                peer_directory = directory.parent / identity
                peer = read_json(peer_directory / "record.json")
                if not peer or peer.get("id") != identity or peer.get("fixture") != installation.id:
                    fail("named peer does not belong to this exact campaign")
                process = ProcessIdentity.from_json(peer["owner"])
                if process.foreign() or fixture.owner_alive(peer["owner"], peer_directory / "owner.lock"):
                    fail("named peer owner is live or unobservable")
                lease = try_hold_lock(peer_directory / "owner.lock")
                if lease is None:
                    fail("named peer owner lease is live")
                peer_locks[identity] = lease
                if read_json(peer_directory / "record.json") != peer:
                    fail("named peer receipt changed")
                peer_rows[identity] = peer
            unnamed = [entry for entry in service.borrowers(installation) if entry.get("id") not in {attachment_id,*peers}]
            if unnamed:
                fail("unnamed unresolved borrowers prevent named recovery")
            use = try_hold_lock(installation.directory / "borrowers.lock")
            if use is None:
                fail("a borrower lease is live")
            if fixture._command_observation(row)["status"] != "gone":
                fail("exact command process cleanup is not confirmed")
            if ((row.get("command_cleanup") or {}).get("status") != "confirmed"
                    or (row.get("command_cleanup") or {}).get("survivors")):
                fail("recorded command cleanup is not confirmed")
            (validate_native_server if native else validate_server)(installation)
            native_files, manifest = (native_inputs if native else inputs)(directory, row)
            scope = ({identity:native_scope_snapshot(directory.parent / identity, peer)
                for identity,peer in {attachment_id:row,**peer_rows}.items()} if native else None)
            if native:
                legacy_ids={identity for identity,snapshot in scope.items() if snapshot["classification"].startswith("historical")}
                if set(legacy_runs or {}) != legacy_ids:
                    fail("every historical peer requires its explicit original run, without unnamed run mappings")
            root = directory / ("native-host-recovery" if native else "pd02-host-recovery")
            plan_path = root / "plan.json"
            if marker:
                plan = json.loads(checked_bytes(plan_path, readonly=True))
                if marker.get("plan_sha256") != service.file_sha256(plan_path):
                    fail("recovery plan changed")
            elif transferred_maintenance is not None and plan_path.exists():
                # A crash after immutable preservation but before marker transfer
                # resumes only that exact prepared scope, never an unclaimed plan.
                plan = json.loads(checked_bytes(plan_path, readonly=True))
                if plan.get("transferred_maintenance") != transferred_maintenance:
                    fail("prepared recovery preserved a different maintenance owner")
                marker = {"operation": operation, "attachment": attachment_id,
                    "plan_sha256": service.file_sha256(plan_path), "phase": "prepared"}
            else:
                if root.exists():
                    fail("unclaimed recovery journal requires its original maintenance marker")
                installation.check_daemon()
                state = service.unit_properties(installation.unit, "InvocationID", "MainPID")
                process = ProcessIdentity.of(int(state["MainPID"]))
                if (not re.fullmatch("[a-f0-9]{32}", state.get("InvocationID", ""))
                        or process != installation._daemon_identity[0]):
                    fail("daemon invocation/process identity differs")
                root.mkdir(mode=0o700)
                plan = {"schema": 1, "kind": operation, "attachment": attachment_id,
                    "fixture": installation.id, "generation": row["native_drain"]["generation"],
                    "original_record": row, "original_record_bytes": base64.b64encode((directory / "record.json").read_bytes()).decode(), "original_record_sha256": service.file_sha256(directory / "record.json"),
                    "native_files": native_files, "manifests": manifest,
                    "descriptor": installation.record, "descriptor_bytes": base64.b64encode((installation.directory / "installation.json").read_bytes()).decode(), "descriptor_sha256": service.file_sha256(installation.directory / "installation.json"),
                    "daemon": {"invocation": state["InvocationID"], "process": process.to_json()},
                    "session_source": "238bfeb11f5725bebed370167656748df8067595 + reviewed grpc-execution-drain.patch: RpcState.sessions is process-local"}
                if native:
                    plan["source_proof"] = SOURCE_JOIN
                    plan["transferred_maintenance"] = transferred_maintenance
                    plan["named_scope"] = scope
                    plan["legacy_runs"] = {identity:legacy_run_snapshot(run_id,scope[identity]["original_record"])
                        for identity,run_id in (legacy_runs or {}).items()}
                    plan["daemon"].update(native_daemon_snapshot(installation, process))
                    plan["native_bytes"] = {name: base64.b64encode(checked_bytes(directory / "native-drain" / name)).decode()
                        for name in native_files}
                immutable(plan_path, plan)
                marker = {"operation": operation, "attachment": attachment_id,
                    "plan_sha256": service.file_sha256(plan_path), "phase": "prepared"}
            if (plan.get("kind") != operation or plan.get("attachment") != attachment_id
                    or plan.get("fixture") != installation.id
                    or plan.get("generation") != row["native_drain"]["generation"]
                    or plan["descriptor"] != installation.record
                    or plan["native_files"] != native_files
                    or plan["manifests"] != manifest):
                fail("original native inputs or daemon descriptor changed")
            if marker.get("phase") not in {"prepared", "joining", "joined"}:
                fail("unknown named recovery checkpoint")
            if native:
                mutable = {"host_recovery", "released", "released_at"}
                if (set(plan["named_scope"]) != {attachment_id,*peers}
                        or not valid_native_scope(directory,plan)
                        or plan.get("source_proof") != SOURCE_JOIN
                        or {key: value for key, value in row.items() if key not in mutable}
                            != {key: value for key, value in plan["original_record"].items() if key not in mutable}
                        or set(plan["native_bytes"]) != set(native_files)
                        or any(digest_bytes(base64.b64decode(value, validate=True)) != native_files[name]
                            for name, value in plan["native_bytes"].items())):
                    fail("original Started receipts or attachment changed before recovery")
            if transferred_maintenance is not None and checked_bytes(marker_path) != base64.b64decode(transferred_maintenance["bytes"], validate=True):
                fail("ordinary maintenance ownership changed before durable transfer")
            marker.update(owner=ProcessIdentity.of().to_json(), token=secrets.token_urlsafe(32))
            durable_json(marker_path, marker)
            maintenance_claimed = True
            installer = Path(installation.record["installer"])
            if marker["phase"] == "prepared":
                service._run_installer(installation, installer, "close-admission")
                marker["phase"] = "joining"
                durable_json(marker_path, marker)
            if marker["phase"] == "joining":
                if not (root / "joined.json").exists():
                    joined = (joined_exit(installation, plan, source_correlated=True)
                        if native else joined_exit(installation, plan))
                    if not valid_join_evidence(joined, plan["daemon"], installation.unit, source_correlated=native):
                        fail("checked server join evidence differs before publication")
                    immutable(root / "joined.json", joined)
                marker["phase"] = "joined"
                durable_json(marker_path, marker)
            joined = json.loads(checked_bytes(root / "joined.json", readonly=True))
            if not valid_join_evidence(joined, plan["daemon"], installation.unit, source_correlated=native):
                fail("retained joined invocation proof differs before restart")
            # Restart only after the exact old registry's selected proof validates.
            service._start_owned(installation)
            installation.check_daemon()
            new_process = installation._daemon_identity[0].to_json()
            if new_process == plan["daemon"]["process"]:
                fail("old invocation was not replaced")
            if native:
                new_state = service.unit_properties(installation.unit, "InvocationID", "MainPID", "ActiveState")
                new_invocation = new_state.get("InvocationID", "")
                if (not re.fullmatch("[a-f0-9]{32}", new_invocation)
                        or new_invocation == plan["daemon"]["invocation"]
                        or new_state.get("MainPID") != str(new_process["pid"])
                        or new_state.get("ActiveState") != "active"):
                    fail("replacement daemon invocation differs from its checked process")
            service._run_installer(installation, installer, "close-admission")
            if not native and not (root / "reconciliation.json").exists():
                immutable(root / "reconciliation.json", reconcile(installation, plan["manifests"], new_process))
            outcomes = None if native else json.loads(checked_bytes(root / "reconciliation.json", readonly=True))
            removed = []
            for table in ([] if native else plan["manifests"]["tables"]):
                terminal = root / f"removed-{table}.json"
                if not terminal.exists():
                    # All IDs come from the validated finite policy, never serialized SQL.
                    installation.check_daemon()
                    result = service.sql(installation, f"REMOVE TABLE IF EXISTS {table}",
                        cfg=installation.runtime("validation", installer=True))
                    if (len(result) != 1 or not isinstance(result[0], dict)
                            or result[0].get("status") != "OK"):
                        fail("owned table removal lacks one checked terminal envelope")
                    immutable(terminal, {"table": table, "terminal": result})
                removed.append(table)
            service._run_installer(installation, installer, "drain")
            installation.check(allow_maintenance=True)
            receipt_path = root / "receipt.json"
            if not receipt_path.exists():
                receipt = {"schema": 1, "kind": operation, "outcome": "recovered",
                    "plan_sha256": service.file_sha256(plan_path), "joined": joined,
                    "new_daemon": new_process, "reconciliation": outcomes, "tables_removed": removed,
                    "cleanup_files": {f"removed-{table}.json": service.file_sha256(root / f"removed-{table}.json") for table in removed},
                    "session_premise": "pinned volatile RpcState; exact joined invocation replaced",
                    "native_drain": "confirmed"}
                if native:
                    receipt.update(source_proof=SOURCE_JOIN,
                        new_invocation=new_invocation,
                        named_scope={identity:snapshot["classification"] for identity,snapshot in plan["named_scope"].items()},
                        recovery_scope="installation-wide persisted-owner recovery under exclusive maintenance",
                        original_request_outcomes="unknown; no original request/attempt attribution available",
                        maintenance_actions=["close-admission", "drain", "check"],
                        reconciliation=None, tables_removed=[], cleanup_files={})
                immutable(receipt_path, receipt)
            row = read_json(directory / "record.json")
            row["host_recovery"] = {"path": str(receipt_path), "sha256": service.file_sha256(receipt_path)}
            if native:
                row["host_recovery"].update(kind=NATIVE_OPERATION,primary=attachment_id)
                # Publish references only after the common terminal receipt exists.
                # Original native status and local release remain independently visible.
                durable_json(directory / "record.json",row)
                for identity in peers:
                    peer=read_json(directory.parent / identity / "record.json")
                    peer["host_recovery"]=row["host_recovery"]
                    durable_json(directory.parent / identity / "record.json",peer)
            if not recovered(directory, row):
                fail("independent host recovery receipt did not validate")
            row.update(released=True, released_at=fixture.now())
            durable_json(directory / "record.json", row)
            fixture._complete_attachment_obligations(directory, "confirmed named host recovery; original native unknown preserved", owner_lease=owner_lock)
            for identity in peers:
                peer_directory=directory.parent / identity
                peer=read_json(peer_directory / "record.json")
                if not native_recovered(peer_directory,peer):
                    fail("independent named peer recovery evidence differs")
                peer.update(released=True,released_at=peer.get("released_at") or fixture.now())
                durable_json(peer_directory / "record.json",peer)
                fixture._complete_attachment_obligations(peer_directory,"confirmed named installation-wide recovery; original native status preserved",owner_lease=peer_locks[identity])
            service._run_installer(installation, installer, "open-admission")
            marker_path.unlink()
            fsync_directory(installation.directory)
            return {"outcome": "recovered", "attachment": attachment_id,
                "original_native_completion": fixture.native_completion(directory, row), "host_recovery": row["host_recovery"]}
        except BaseException:
            if maintenance_claimed and marker_path.exists():
                with contextlib.suppress(Exception):
                    service._run_installer(installation, Path(installation.record["installer"]), "close-admission")
            raise
        finally:
            for fd in (use, *peer_locks.values(), owner_lock, gate):
                if fd is not None:
                    os.close(fd)


def prepare_logs(installation):
    """Explicit campaign prerequisite; this ordinary restart proves no lost RPC finality."""
    operation = "PD02 campaign log preparation"
    if installation.record.get("kind") != "campaign":
        fail("positive shutdown logging preparation is campaign-only")
    generation = installation.record["binary"].get("generation")
    if (not generation or generation["provenance"]["recipe"]["pin"]["build_metadata"]
            != "lctx.timeout-stack.grpc-drain.1"):
        fail("campaign is not the reviewed registered-drain generation")
    with admission([installation.directory]):
        gate = try_hold_lock(installation.directory / "admission.lock")
        if gate is None:
            fail("another admission/maintenance operation is live")
        use = None
        marker_path = installation.directory / "maintenance.json"
        try:
            marker = read_json(marker_path)
            if marker and (marker.get("operation") != operation
                    or ProcessIdentity.from_json(marker["owner"]).alive()):
                fail("another or live maintenance operation is unresolved")
            if service.borrowers(installation):
                fail("logging preparation requires all borrowers resolved")
            use = try_hold_lock(installation.directory / "borrowers.lock")
            if use is None:
                fail("logging preparation borrower lease is live")
            binary = service._installed_binary(installation)
            if marker:
                path = Path(marker["plan"])
                if (path.parent.parent != installation.directory / "service-log-preparations"
                        or service.file_sha256(path) != marker["plan_sha256"]):
                    fail("logging preparation plan identity differs")
                plan = json.loads(checked_bytes(path, readonly=True))
            else:
                if installation.record.get("server_log_filter", "warn") != "warn":
                    fail("positive shutdown logging is already prepared; observe its existing receipt")
                installation.check_daemon()
                port = int(installation.endpoint.rsplit(":", 1)[1])
                previous = service.unit_text(installation.directory, binary, port)
                if service._unit_path(installation.unit).read_text() != previous:
                    fail("campaign unit differs before logging preparation")
                root = installation.directory / "service-log-preparations" / secrets.token_hex(32)
                root.mkdir(parents=True, mode=0o700)
                path = root / "plan.json"
                successor = dict(installation.record)
                successor["server_log_filter"] = "warn,surrealdb::net=info"
                successor["server_log_preparation"] = {"plan": str(path)}
                plan = {"kind": operation, "previous_descriptor": installation.record,
                    "successor_descriptor": successor, "previous_unit": previous,
                    "successor_unit": service.unit_text(installation.directory, binary, port,
                        log_filter="warn,surrealdb::net=info")}
                immutable(path, plan)
                marker = {"operation": operation, "plan": str(path), "plan_sha256": service.file_sha256(path), "phase": "prepared"}
            if (installation.record not in (plan["previous_descriptor"], plan["successor_descriptor"])
                    or service._unit_path(installation.unit).read_text() not in
                        (plan["previous_unit"], plan["successor_unit"])):
                fail("campaign descriptor/unit changed outside logging journal")
            marker.update(owner=ProcessIdentity.of().to_json(), token=secrets.token_urlsafe(32))
            durable_json(marker_path, marker)
            installer = Path(installation.record["installer"])
            if marker["phase"] == "prepared":
                service._run_installer(installation, installer, "close-admission")
                service._run_installer(installation, installer, "drain")
                marker["phase"] = "drained"
                durable_json(marker_path, marker)
            if marker["phase"] == "drained":
                # Ordinary pre-request maintenance only. Never used to recover
                # lost wire finality: its stop timeout is not joined_exit.
                service._stop_owned(installation)
                service._write_private(service._unit_path(installation.unit), plan["successor_unit"])
                if service.systemctl("daemon-reload").returncode:
                    fail("logging preparation daemon-reload failed")
                installation.record = plan["successor_descriptor"]
                durable_json(installation.directory / "installation.json", installation.record)
                marker["phase"] = "installed"
                durable_json(marker_path, marker)
            if installation.record != plan["successor_descriptor"]:
                fail("logging preparation installed descriptor differs")
            smoke_plan_path = path.parent / "empty-registry-plan.json"
            smoke_joined_path = path.parent / "empty-registry-joined.json"
            if marker["phase"] == "installed":
                service._start_owned(installation)
                service._run_installer(installation, installer, "close-admission")
                service._run_installer(installation, installer, "drain")
                installation.check_daemon()
                state = service.unit_properties(installation.unit, "InvocationID", "MainPID")
                process = ProcessIdentity.of(int(state["MainPID"]))
                if (not re.fullmatch("[a-f0-9]{32}", state.get("InvocationID", ""))
                        or process != installation._daemon_identity[0]):
                    fail("positive shutdown smoke daemon/invocation differs")
                smoke = {"daemon": {"invocation": state["InvocationID"], "process": process.to_json()},
                    "binary": installation.record["binary"], "unit": installation.unit,
                    "claim": "empty registry prerequisite; no abandoned-request finality"}
                if smoke_plan_path.exists():
                    if json.loads(checked_bytes(smoke_plan_path, readonly=True)) != smoke:
                        fail("positive shutdown smoke changed before checkpoint")
                else:
                    immutable(smoke_plan_path, smoke)
                marker["phase"] = "smoke_joining"
                durable_json(marker_path, marker)
            smoke = json.loads(checked_bytes(smoke_plan_path, readonly=True))
            if smoke["binary"] != installation.record["binary"]:
                fail("positive shutdown smoke binary changed")
            if marker["phase"] == "smoke_joining":
                if not smoke_joined_path.exists():
                    joined = joined_exit(installation, smoke)
                    if not valid_join_evidence(joined, smoke["daemon"], installation.unit):
                        fail("positive shutdown smoke evidence differs before publication")
                    immutable(smoke_joined_path, joined)
                marker["phase"] = "smoke_joined"
                durable_json(marker_path, marker)
            # The exact empty-registry old invocation has joined; now restore the
            # same checked server generation while admission is still closed.
            service._start_owned(installation)
            service._run_installer(installation, installer, "close-admission")
            service._run_installer(installation, installer, "drain")
            installation.check(allow_maintenance=True)
            receipt_path = path.parent / "receipt.json"
            if not receipt_path.exists():
                immutable(receipt_path, {"kind": operation, "outcome": "prepared",
                    "plan_sha256": service.file_sha256(path), "binary": installation.record["binary"],
                    "log_filter": installation.record["server_log_filter"],
                    "registry_join_evidence": "empty registry prerequisite only; abandoned-request finality not_observed",
                    "empty_registry_plan_sha256": service.file_sha256(smoke_plan_path),
                    "empty_registry_join": json.loads(checked_bytes(smoke_joined_path, readonly=True))})
            service._run_installer(installation, installer, "open-admission")
            marker_path.unlink()
            fsync_directory(installation.directory)
            return {"outcome": "prepared", "receipt": str(receipt_path),
                "receipt_sha256": service.file_sha256(receipt_path), "binary": installation.record["binary"]["sha256"]}
        except BaseException:
            if marker_path.exists():
                with contextlib.suppress(Exception):
                    service._run_installer(installation, Path(installation.record["installer"]), "close-admission")
            raise
        finally:
            for fd in (use, gate):
                if fd is not None:
                    os.close(fd)
