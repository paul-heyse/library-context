"""Explicit regeneration of the named design campaign after joined daemon shutdown.

This disposition does not reconcile an interrupted native outcome. Its unknown
outcome stays immutable evidence; only the explicitly discarded database scopes
are replaced with the current schema under the installation maintenance owner.
"""
from __future__ import annotations
import base64
import contextlib
import fcntl
import hashlib
import json
import os
import re
import secrets
import stat
from pathlib import Path

import surrealdb_service as service
from harness import ProcessIdentity, read_json, try_hold_lock
from storage_lifecycle import admission, fsync_directory

OPERATION = "design campaign regeneration"
CAMPAIGN = "0ec1d0c0-fbd0-4265-93e5-d9e186bab099"
ENDPOINT = "http://127.0.0.1:33385"
ATTACHMENT = "fcad900d-aa6b-4e98-aed0-56154d4216dc"
RUN = "20261012T071909.321Z-e5da48"


def fail(detail):
    raise service.FixtureBlocked("campaign regeneration", detail,
        "resume the exact --regenerate-campaign --installer PATH --discard-attachment ID --discard-run PATH")


def owned_bytes(path):
    path = Path(path)
    if any(p.is_symlink() for p in (path, *path.parents)):
        fail("evidence path is not physical")
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or before.st_uid != os.getuid() or before.st_mode & 0o002:
            fail("evidence is not an owned regular file")
        with os.fdopen(fd, "rb", closefd=False) as stream:
            data = stream.read()
        if service.executable_identity(before) != service.executable_identity(os.fstat(fd)):
            fail("evidence changed while observed")
        return data
    finally:
        os.close(fd)


def evidence(path):
    raw = owned_bytes(path)
    return {"path": str(path), "sha256": hashlib.sha256(raw).hexdigest()}


def immutable(path, value):
    from surrealdb_pd02_recovery import immutable as publish
    if path.exists():
        if json.loads(owned_bytes(path)) != value or stat.S_IMODE(path.stat().st_mode) != 0o400:
            fail("immutable disposition differs")
    else:
        publish(path, value)


def read_immutable(path):
    raw = owned_bytes(path)
    if stat.S_IMODE(path.stat().st_mode) != 0o400:
        fail("immutable disposition permissions differ")
    return json.loads(raw)


def runtime_preflight(installation, runtime):
    for db in service.DATABASES:
        cfg = runtime[db]
        for key in ("username", "viewer_username"):
            if not isinstance(cfg.get(key), str) or not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", cfg[key]):
                fail("runtime username is not a simple owned identifier")
        selected = Path(cfg["selection"])
        leases = Path(cfg["reuse"]["lease_directory"])
        if (selected != installation.directory / f"{db}-selected.json" or selected.is_symlink()
                or leases != installation.directory / "product-leases" / db
                or any(path.is_symlink() for path in (leases, *leases.parents))):
            fail("runtime selection or lease directory escaped its discarded scope")
        # These synchronization files carry no old semantic data. Any actual
        # payload requires a separate explicit disposition rather than guessing.
        if leases.exists():
            for path in leases.iterdir():
                info = path.lstat()
                if (path.name not in {"lifecycle.lock", "admission.lock"}
                        or not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid()):
                    fail("discarded product lease directory has unclassified content")


def scope(installation, attachment, run):
    if (installation.record.get("kind") != "campaign" or installation.id != CAMPAIGN
            or installation.endpoint != ENDPOINT
            or installation.record.get("namespace") != service.NAMESPACE
            or installation.record.get("databases") != list(service.DATABASES)
            or installation.unit != service.installation_unit(installation.record)
            or attachment != ATTACHMENT or run.name != RUN):
        fail("only the explicitly named campaign, attachment and run may be regenerated")
    old = service.ROOT.with_name("library-context-wt") / "efficiency-source5"
    if run != old / "build/runs" / RUN:
        fail("discarded run is outside its producing checkout")
    return old


def source_inputs(installation, attachment, run, marker):
    import surrealdb_fixture as fixture
    old = scope(installation, attachment, run)
    record_path = installation.directory / "attachments" / attachment / "record.json"
    original = json.loads(owned_bytes(record_path))
    retained_run = json.loads(owned_bytes(run / "record.json"))
    if (retained_run.get("id") != RUN or retained_run.get("cwd") != str(old)
            or retained_run.get("argv") != ["env", "LCTX_SURREAL_SERVICE_CONFIG=" + str(installation.directory / "installation.json"),
                "uv", "run", "--no-project", "--offline", "--no-python-downloads", "--python", "3.14.7",
                "scripts/efficiency_source5_upper.py", "--produce"]
            or retained_run.get("phase") != "ended" or retained_run.get("termination") != "cancelled"
            or retained_run.get("exit") != {"code": 143}
            or retained_run.get("cleanup", {}).get("status") != "confirmed"
            or retained_run.get("cleanup", {}).get("survivors") != []):
        fail("named interrupted run lacks confirmed descendant cleanup")
    if (original.get("id") != attachment or original.get("checkout") != str(service.ROOT)
            or original.get("maintenance") is not True or original.get("released") is not False
            or marker.get("operation") != "maintenance" or marker.get("owner") != original.get("owner")):
        fail("original attachment and plain maintenance owner do not correspond")
    owner = ProcessIdentity.from_json(original["owner"])
    if owner.foreign() or owner.alive() or fixture._command_observation(original).get("status") != "gone":
        fail("original command owner or descendants are live or unobservable")
    native = original.get("native_drain", {})
    native_root = record_path.parent / "native-drain"
    if (native.get("attachment") != attachment or native.get("directory") != str(native_root)
            or native.get("generation") != bytes(installation.record["service_generation"]).hex()
            or not native_root.is_dir() or native_root.is_symlink() or list(native_root.iterdir())):
        fail("original native receipt directory is not the exact observed empty inventory")
    producer_path = old / ".campaign/source5/upper-producer.json"
    producer = json.loads(owned_bytes(producer_path))
    paths = {"source": old / "crates/lctx-publisher/tests/efficiency_source5_upper.rs",
        "seed_source": old / "crates/lctx-publisher/tests/efficiency_source5_seed.rs",
        "executable": Path(producer["executable"])}
    inputs = [evidence(producer_path), evidence(run / "record.json"), evidence(old / "scripts/efficiency_source5_upper.py")]
    for key, path in paths.items():
        observed = evidence(path)
        if observed["sha256"] != producer.get(key + "_sha256"):
            fail("original producer source or executable differs from its retained receipt")
        inputs.append(observed)
    argv = original.get("command", {}).get("argv", [])
    if (str(paths["executable"]) not in argv or argv[-4:] !=
            ["--ignored", "--exact", "populate_genuine_source5_upper_artifact", "--nocapture"]):
        fail("original child selector differs")
    for path in retained_run.get("logs", {}).values():
        path = Path(path)
        if path.parent != run:
            fail("run evidence escaped its exact directory")
        inputs.append(evidence(path))
    return {"attachment": original, "maintenance": marker, "run": retained_run,
        "sources": inputs, "original_native_outcome": "unknown"}


def recheck_sources(plan):
    for expected in plan["original"]["sources"]:
        if evidence(Path(expected["path"])) != expected:
            fail("immutable original evidence changed")


def ns_databases(installation, config):
    values = service.sql(installation, "INFO FOR NS;", cfg={**config, "database": None})
    result = values[0].get("result") if len(values) == 1 else None
    if not isinstance(result, dict) or not isinstance(result.get("databases"), dict):
        fail("independent namespace database inventory is unavailable")
    return set(result["databases"])


def validate_completed(installation, plan, *, require_closed=True):
    for db in service.DATABASES:
        rows = service.sql(installation,
            "SELECT generation,schema,schema_version,admission_open FROM native_installation:current;",
            cfg=installation.runtime(db, installer=True))
        expected = {"generation": bytes(plan["generation"]).hex(), "schema": plan["target"]["schema"],
            "schema_version": 6, "admission_open": False}
        if not require_closed:
            actual = rows[0].get("result") if len(rows) == 1 else None
            if (not isinstance(actual, list) or len(actual) != 1 or not isinstance(actual[0], dict)
                    or type(actual[0].get("admission_open")) is not bool):
                fail("current initialized admission marker is unavailable")
            expected["admission_open"] = actual[0]["admission_open"]
        if len(rows) != 1 or rows[0].get("result") != [expected]:
            fail("current initialized scope differs from exact closed target")
        service._installer_cli(installation, plan["candidate"], db, "check")


def checked_disposition(installation, plan, plan_path, join_path):
    return {"installation_id": installation.id, "generation": plan["generation"],
        "target": plan["target"], "databases": list(service.DATABASES), "original_native_outcome": "unknown",
        "disposition": "explicitly discarded and regenerated", "plan": evidence(plan_path), "joined": evidence(join_path)}


def regenerate(installation, installer, attachment, run):
    from storage_service import prepare
    from surrealdb_pd02_recovery import (validate_native_server, joined_exit,
        valid_join_evidence, native_daemon_snapshot, SOURCE_JOIN)
    import surrealdb_fixture as fixture
    installer, run = Path(installer).absolute(), Path(run).absolute()
    scope(installation, attachment, run)
    with admission([installation.directory, installer, run, service.ROOT,
            service.ROOT.with_name("library-context-wt") / "efficiency-source5"]):
        with contextlib.ExitStack() as stack:
            for path in [installation.directory / "admission.lock", installation.directory / "borrowers.lock",
                    installation.directory / "attachments" / attachment / "owner.lock"]:
                fd = try_hold_lock(path)
                if fd is None:
                    fail("another maintenance, borrower or named attachment owner holds a lease")
                stack.callback(os.close, fd)
            marker_path = installation.directory / "maintenance.json"
            marker = read_json(marker_path)
            if not isinstance(marker, dict) or ProcessIdentity.from_json(marker["owner"]).alive():
                fail("regeneration requires the dead exact maintenance owner")
            pending = service.borrowers(installation)
            if {row.get("id") for row in pending} - {attachment}:
                fail("an unnamed unresolved borrower remains")
            directory = installation.directory / "campaign-regeneration" / attachment
            directory.mkdir(parents=True, exist_ok=True, mode=0o700)
            plan_path = directory / "plan.json"
            if plan_path.exists():
                plan = read_immutable(plan_path)
                if (plan.get("installation_id") != installation.id or plan.get("run") != str(run)
                        or plan.get("source_installer") != str(installer)
                        or not ((marker.get("operation") == OPERATION and marker.get("regeneration") == evidence(plan_path))
                            or marker == plan["original"]["maintenance"])):
                    fail("resume differs from its immutable campaign disposition")
                recheck_sources(plan)
            else:
                original = source_inputs(installation, attachment, run, marker)
                validate_native_server(installation)
                installation.check_daemon()
                state = service.unit_properties(installation.unit, "InvocationID", "MainPID", "ActiveState")
                if state.get("ActiveState") != "active" or not re.fullmatch(r"[0-9a-f]{32}", state.get("InvocationID", "")):
                    fail("exact active daemon invocation is unavailable")
                process = ProcessIdentity.of(int(state["MainPID"]))
                daemon = {"invocation": state["InvocationID"], "process": process.to_json(),
                    "snapshot": native_daemon_snapshot(installation, process)}
                candidate = prepare(installation.directory, installer)
                target = service._installer_cli(installation, candidate, "main", "schema")
                service._current_schema(target)
                generation = list(secrets.token_bytes(32))
                runtime, root = {}, {}
                for db in service.DATABASES:
                    runtime[db] = {**installation.runtime(db), "service_generation": generation,
                        "password": secrets.token_urlsafe(32), "viewer_password": secrets.token_urlsafe(32)}
                    root[db] = {**runtime[db], **{key: installation.runtime(db, installer=True)[key]
                        for key in ("username", "password", "authentication")}}
                plan = {"schema": 1, "operation": OPERATION, "installation_id": installation.id,
                    "attachment": attachment, "run": str(run), "source_installer": str(installer),
                    "original": original, "installation": installation.record, "daemon": daemon,
                    "source_join": SOURCE_JOIN, "candidate": candidate, "target": target,
                    "generation": generation, "runtime": runtime, "root": root}
                runtime_preflight(installation, runtime)
                immutable(plan_path, plan)
                plan = read_immutable(plan_path)
                service.private_json(marker_path, {"operation": OPERATION, "owner": ProcessIdentity.of().to_json(),
                    "token": secrets.token_urlsafe(32), "regeneration": evidence(plan_path)})
            if service.file_sha256(installer) != plan["candidate"]["sha256"]:
                fail("source installer changed since its immutable candidate was prepared")
            runtime_preflight(installation, plan["runtime"])
            # Prove old product readers drained through scope removal. Current
            # installation subsequently acquires these same locks itself.
            product_leases = stack.enter_context(contextlib.ExitStack())
            for db in service.DATABASES:
                for name in ("admission.lock", "lifecycle.lock"):
                    path = installation.directory / "product-leases" / db / name
                    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
                    fd = try_hold_lock(path)
                    if fd is None:
                        fail("an actual product lifecycle lease remains held")
                    product_leases.callback(os.close, fd)
            # Every stage remains under the same locks and storage admission. Unknown
            # cleanup preserves the plan and closed host marker for exact resume.
            recheck_sources(plan)
            service.private_json(marker_path, {"operation": OPERATION, "owner": ProcessIdentity.of().to_json(),
                "token": secrets.token_urlsafe(32), "regeneration": evidence(plan_path)})
            join_path = directory / "joined.json"
            if join_path.exists():
                joined = read_immutable(join_path)
            else:
                if installation.record != plan["installation"]:
                    fail("original installation descriptor changed before daemon join")
                validate_native_server(installation)
                state = service.unit_properties(installation.unit, "InvocationID", "MainPID", "ActiveState")
                if state.get("ActiveState") == "active":
                    installation.check_daemon()
                    original_process = ProcessIdentity.from_json(plan["daemon"]["process"])
                    if native_daemon_snapshot(installation, original_process) != plan["daemon"]["snapshot"]:
                        fail("actual original daemon argv or executable changed before join")
                elif (state.get("ActiveState") != "inactive" or state.get("MainPID") != "0"
                        or state.get("InvocationID") != plan["daemon"]["invocation"]):
                    fail("original daemon is neither the checked active process nor its exact inactive invocation")
                joined = joined_exit(installation, plan, source_correlated=True)
                immutable(join_path, joined)
            if not valid_join_evidence(joined, plan["daemon"], installation.unit, source_correlated=True):
                fail("exact daemon did not confirm registered execution join")
            # The original run/attachment outcome is not changed by this local cleanup.
            original = plan["original"]["attachment"]
            if fixture._command_observation(original).get("status") != "gone":
                fail("original local descendants are no longer proved gone")
            service._owned_daemon_start(installation)
            old_root = plan["root"]["main"]
            databases = ns_databases(installation, old_root)
            if databases - set(service.DATABASES):
                fail("campaign namespace has an unapproved database")
            for db in service.DATABASES:
                removed = directory / (db + "-removed.json")
                expected_removed = {"database": db, "removed": True, "join": evidence(join_path)}
                if removed.exists():
                    if read_immutable(removed) != expected_removed:
                        fail("discarded database receipt differs from its checked daemon join")
                else:
                    # A lost remove acknowledgement is resolved by actual namespace
                    # readback before writing the immutable removal receipt.
                    if db in ns_databases(installation, old_root):
                        try:
                            service.sql(installation, f"REMOVE DATABASE {db};", cfg={**old_root, "database": None})
                        except (service.FixtureBlocked, service.FixtureFailed):
                            if db in ns_databases(installation, old_root):
                                raise
                    if db in ns_databases(installation, old_root):
                        fail("discarded scope removal is not independently confirmed")
                    immutable(removed, expected_removed)
            product_leases.close()
            # Runtime configuration changes only after both exact old scopes are gone.
            for db in service.DATABASES:
                service.private_json(installation.runtime_path(db), plan["runtime"][db])
                service.private_json(installation.runtime_path(db, installer=True), plan["root"][db])
                selected = Path(plan["runtime"][db]["selection"])
                if selected != installation.directory / f"{db}-selected.json" or selected.is_symlink():
                    fail("selection pointer is outside its explicitly discarded scope")
                selected.unlink(missing_ok=True)
            installation.record.update(service_generation=plan["generation"], installer=plan["candidate"]["path"],
                installer_generation=plan["candidate"], native_schema_version=6, phase="regenerating",
                campaign_regeneration=evidence(plan_path))
            service.private_json(installation.directory / "installation.json", installation.record)
            checked_path = directory / "checked.json"
            if checked_path.exists():
                # A process may die after reopening but before publishing complete.json.
                # Authenticate the actual current generation and immutable candidate
                # before closing it; removed receipts prevent any second discard.
                if read_immutable(checked_path) != checked_disposition(installation, plan, plan_path, join_path):
                    fail("checked disposition differs before resume admission fence")
                if service._installer_cli(installation, plan["candidate"], "main", "schema") != plan["target"]:
                    fail("current installer schema changed before resume admission fence")
                validate_completed(installation, plan, require_closed=False)
                service._run_installer(installation, Path(plan["candidate"]["path"]), "close-admission")
            for db in service.DATABASES:
                cfg = plan["runtime"][db]
                for key in ("username", "viewer_username"):
                    if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", cfg[key]):
                        fail("runtime username is not a simple owned identifier")
                service.sql(installation, f"DEFINE DATABASE IF NOT EXISTS {db} STRICT; USE DB {db}; "
                    f"DEFINE USER OVERWRITE {cfg['username']} ON DATABASE PASSWORD {json.dumps(cfg['password'])} ROLES OWNER; "
                    f"DEFINE USER OVERWRITE {cfg['viewer_username']} ON DATABASE PASSWORD {json.dumps(cfg['viewer_password'])} ROLES VIEWER;",
                    cfg={**plan["root"][db], "database": None})
                service._installer_cli(installation, plan["candidate"], db, "init", keep_closed=True)
            service._run_installer(installation, Path(plan["candidate"]["path"]), "close-admission")
            validate_completed(installation, plan)
            checked = checked_disposition(installation, plan, plan_path, join_path)
            immutable(directory / "checked.json", checked)
            attachment_path = installation.directory / "attachments" / attachment / "record.json"
            actual = json.loads(owned_bytes(attachment_path))
            if actual != original and actual.get("campaign_disposition") != evidence(directory / "checked.json"):
                fail("original attachment changed outside its checked disposition")
            service.private_json(attachment_path, {**original, "released": True,
                "command_cleanup": {"status": "confirmed", "survivors": []},
                "campaign_disposition": evidence(directory / "checked.json")})
            # Existing readiness, exact definition checks and both closed scopes precede
            # reopening; a failed open immediately attempts to close both scopes again.
            installation.record["phase"] = "ready"
            service.private_json(installation.directory / "installation.json", installation.record)
            installation.check(allow_maintenance=True)
            try:
                service._run_installer(installation, Path(plan["candidate"]["path"]), "open-admission")
            except BaseException:
                with contextlib.suppress(Exception):
                    service._run_installer(installation, Path(plan["candidate"]["path"]), "close-admission")
                raise
            immutable(directory / "complete.json", {**checked, "admission_open": True})
            marker_path.unlink(); fsync_directory(installation.directory)
            return {"outcome": "passed", "installation_id": installation.id,
                "databases": list(service.DATABASES), "original_native_outcome": "unknown",
                "disposition": str(directory / "complete.json"), "admission_open": True}


def discarded(directory, record):
    """An explicit scope disposition releases protection, never proves old finality."""
    try:
        reference = record.get("campaign_disposition", {})
        installation = directory.parent.parent
        expected = installation / "campaign-regeneration" / directory.name / "checked.json"
        if directory.name != ATTACHMENT or reference != evidence(expected):
            return False
        checked = read_immutable(expected)
        plan_path = expected.with_name("plan.json")
        plan = read_immutable(plan_path)
        joined = read_immutable(expected.with_name("joined.json"))
        from surrealdb_pd02_recovery import valid_join_evidence
        return (checked.get("plan") == evidence(plan_path)
            and checked.get("installation_id") == CAMPAIGN
            and checked.get("databases") == list(service.DATABASES)
            and checked.get("original_native_outcome") == "unknown"
            and checked.get("disposition") == "explicitly discarded and regenerated"
            and checked.get("joined") == evidence(expected.with_name("joined.json"))
            and plan.get("attachment") == ATTACHMENT
            and valid_join_evidence(joined, plan["daemon"], service.installation_unit(plan["installation"]), source_correlated=True)
            and record.get("native_drain") == plan["original"]["attachment"].get("native_drain"))
    except (OSError,ValueError,TypeError,KeyError,service.FixtureBlocked):
        return False
