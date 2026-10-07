"""Private bounded numerical replay from one pinned native cohort, without inference.

The ordinary CLI supplies its checked VIEWER/read-only boundary. This adapter reads
canonical winning bytes and actual indexed ANN nominations; it never supplies
expected answers to production or labels a bounded cohort as the whole library.
"""

from __future__ import annotations

import base64
import json
import math
import os
import re
import struct
import subprocess
import tempfile
from pathlib import Path
from typing import Any

from programmatic_eval import Worker, WorkerError, stored_vector_reference


class NativeReference:
    """Freeze an existing viewer selection for the lifetime of this private replay."""

    def __init__(self, binary: Path, viewer: Path):
        self.binary = binary.resolve()
        configured = json.loads(viewer.read_text())
        self.snapshot = json.loads(Path(configured["selection"]).read_text())
        self._scratch = tempfile.TemporaryDirectory(prefix="lctx-private-numeric-")
        root = Path(self._scratch.name)
        self.handle = root / "handle.json"
        self.runtime = root / "runtime.json"
        # Snapshot Query never connects with the installer credentials. No root
        # credential enters this adapter; only the existing VIEWER is usable.
        runtime = {
            "endpoint": configured["endpoint"],
            "username": "private_reference_unused_installer",
            "password": "unused",
            "viewer_username": configured["username"],
            "viewer_password": configured["password"],
            "namespace": self.snapshot["database"]["namespace"],
            "cache_database": "unused",
            "selection": str(self.handle),
        }
        for path, value in ((self.handle, self.snapshot), (self.runtime, runtime)):
            descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(descriptor, "w") as stream:
                json.dump(value, stream)

    def rows(self, sql: str) -> list[dict[str, Any]]:
        try:
            result = subprocess.run(
                [
                    str(self.binary),
                    "snapshot",
                    "--runtime-config",
                    str(self.runtime),
                    "query",
                    sql,
                    "--handle",
                    str(self.handle),
                ],
                check=False,
                capture_output=True,
                timeout=30,
            )
        except subprocess.TimeoutExpired as error:
            raise WorkerError("native reference query timed out") from error
        if result.returncode:
            # Runtime/SQL diagnostics may contain sensitive configuration. Keep
            # them out of experiment rows and exception messages.
            raise WorkerError("native reference query failed")
        if len(result.stdout) > 8 << 20:
            raise WorkerError("native reference response exceeds bounded replay")
        statements = json.loads(result.stdout)
        if len(statements) != 1 or not isinstance(statements[0], list):
            raise WorkerError("native reference requires one completed row statement")
        return statements[0]

    def close(self) -> None:
        self._scratch.cleanup()

    def __enter__(self) -> NativeReference:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def _quoted(value: str) -> str:
    return json.dumps(value, ensure_ascii=True)


def _full(payload: str) -> list[float]:
    canonical = json.loads(base64.b64decode(payload, validate=True))
    record = canonical.get("EmbeddingFullValue")
    if not isinstance(record, dict) or record.get("dimensions") != 4096 or record.get("codec") != 1:
        raise WorkerError("native reference requires canonical full4096 F32 values")
    raw = bytes(record["bytes"])
    if len(raw) != 4096 * 4:
        raise WorkerError("native full value has incompatible byte length")
    values = list(struct.unpack("<4096f", raw))
    if not all(math.isfinite(value) for value in values):
        raise WorkerError("native full value is nonfinite")
    return values


def native_numeric_reference(
    worker: Worker,
    native: NativeReference,
    *,
    k: int = 4,
    cohort: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Exhaust one admitted eligible cohort of <=256 rows and compare actual ANN.

    A stored document winner is the query for this numerical control. This tests
    index/projection/rescore mechanics and declares no semantic query relevance.
    Large cohorts refuse rather than silently turning an arbitrary prefix into
    an exhaustive reference. Canonical values are read in 32-key batches.
    """
    if not 1 <= k <= 64:
        raise WorkerError("unsupported native reference k")
    if cohort is None:
        found = native.rows(
            "SELECT <string>id AS vector_id, encoder_hash, policy_key, library_input, family "
            "FROM vector WHERE array::len((SELECT VALUE id FROM vec_occurs "
            "WHERE in=$parent.id AND eligible=true LIMIT 1))>0 ORDER BY vector_id LIMIT 1"
        )
        if not found:
            raise WorkerError("native reference has no eligible admitted vector cohort")
        cohort = {
            key: found[0][key] for key in ("encoder_hash", "policy_key", "library_input", "family")
        }
    if set(cohort) != {"encoder_hash", "policy_key", "library_input", "family"}:
        raise WorkerError("invalid native reference cohort")
    if (
        any(
            not isinstance(cohort[key], str)
            for key in ("encoder_hash", "policy_key", "library_input")
        )
        or type(cohort["family"]) is not int
    ):
        raise WorkerError("invalid native reference cohort values")
    predicates = (
        " AND ".join(
            f"{key}={_quoted(cohort[key])}"
            for key in ("encoder_hash", "policy_key", "library_input")
        )
        + f" AND family={cohort['family']}"
    )
    eligibility = (
        "array::len((SELECT VALUE id FROM vec_occurs WHERE in=$parent.id AND eligible=true "
        f"AND family={cohort['family']} AND scope_input={_quoted(cohort['library_input'])} LIMIT 1))>0"
    )
    rows = native.rows(
        "SELECT <string>id AS vector_id, full_key, projection_key, embedding FROM vector "
        f"WHERE {predicates} AND {eligibility} ORDER BY vector_id LIMIT 257"
    )
    if not rows or len(rows) > 256:
        raise WorkerError("native reference cohort empty or exceeds exhaustive bound256")
    keys = sorted({row["full_key"] for row in rows})
    if any(not re.fullmatch(r"[0-9a-f]{32}", key) for key in keys):
        raise WorkerError("invalid canonical full key")
    full: dict[str, list[float]] = {}
    for start in range(0, len(keys), 32):
        selected = json.dumps(keys[start : start + 32])
        batch = native.rows(
            "SELECT semantic_key, encoding::base64::encode(canonical,true) AS canonical FROM entity "
            f"WHERE semantic_type='embedding_full_values' AND semantic_key IN {selected} ORDER BY semantic_key"
        )
        for record in batch:
            key = record["semantic_key"]
            if key not in keys[start : start + 32] or key in full:
                raise WorkerError("duplicate or foreign canonical full row")
            full[key] = _full(record["canonical"])
    if set(full) != set(keys):
        raise WorkerError("native reference is missing canonical winning values")
    ids = [row["vector_id"] for row in rows]
    if len(set(ids)) != len(ids):
        raise WorkerError("duplicate native cohort row")
    # Independent scalar projection checks that the indexed representation really
    # is the declared1024 prefix of the retained full winner, with one F32 rounding.
    for row in rows:
        prefix = full[row["full_key"]][:1024]
        length = math.sqrt(sum(value * value for value in prefix))
        if not length or len(row["embedding"]) != 1024:
            raise WorkerError("native reference has invalid projected values")
        projected = [struct.unpack("<f", struct.pack("<f", value / length))[0] for value in prefix]
        if row["embedding"] != projected:
            raise WorkerError("native indexed projection differs from canonical winning bytes")
    query = full[rows[0]["full_key"]]
    ann = native.rows(
        "SELECT <string>id AS vector_id, vector::distance::knn() AS distance FROM vector "
        f"WHERE {predicates} AND {eligibility} AND embedding <|{k},128|> "
        f"{json.dumps(rows[0]['embedding'], allow_nan=False)}"
    )
    nominated = [row["vector_id"] for row in ann]
    if (
        not nominated
        or len(set(nominated)) != len(nominated)
        or any(key not in ids for key in nominated)
    ):
        raise WorkerError("native ANN returned empty, duplicate or foreign scope")
    reference = stored_vector_reference(
        worker,
        {
            "policy": {
                "full_dimensions": 4096,
                "projection_dimensions": 1024,
                "block_rows": 32,
                "k": k,
            },
            "query": query,
            "vectors": [
                {"id": row["vector_id"], "values": full[row["full_key"]], "eligible": True}
                for row in rows
            ],
            "nominated_ids": nominated,
        },
    )
    return {
        "snapshot": native.snapshot,
        "cohort": cohort,
        "population_scope": "complete eligible admitted rows in this declared native cohort",
        "query_basis": "retained document full4096 winner, numerical control only",
        "inference_started": False,
        "native_ann": ann,
        "native_nomination_basis": "actual indexed HNSW1024, residual eligible occurrence, k/ef128",
        "reference": reference,
    }
