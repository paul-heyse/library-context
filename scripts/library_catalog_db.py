"""SQLite store for the library-utilization catalog: the usage index and the queries over it.

Two sources, one query surface:

- The tracked catalog, `docs/library-utilization.jsonl` (curated capabilities and generated
  library facts), is always loaded into an in-memory database when a `Store` opens. It is 157
  records, so this takes milliseconds and can never disagree with the file.
- The **usage index**, `build/library-usage.sqlite` (gitignored, written by a full `--write` run),
  holds what the file cannot: every library item the workspace source references (`usage`), the
  mtime and size of every scanned source file when it was indexed (`file_state`), which packages
  count as which library (`library_packages`) and facts about the run (`meta`). It is attached
  read-only when it exists.

Nothing here decides what a lookup *means*: `library_catalog_hints` does that. Queries return every
match; there is no result cap.
"""

from __future__ import annotations

import datetime
import hashlib
import json
import os
import re
import sqlite3
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import library_scan as scan
import library_semantic as semantic

INDEX = Path("build/library-usage.sqlite")
JSONL = Path("docs/library-utilization.jsonl")
SCHEMA_VERSION = 1

# What the usage index cannot see, stated with every answer that rests on its absence.
BLIND_SPOTS = (
    "references inside macro expansions and derive or attribute output are not indexed "
    "(the proc-macro server is off)",
    "methods that a proc macro generates (for example enum_dispatch) are not indexed",
    "files outside the module tree (for example a tests subdirectory that no target includes) "
    "are not indexed",
    "a type that is only inferred and never named in a path is not recorded "
    "(its methods are, when the call resolves)",
)

INDEX_SCHEMA = """
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE usage (
    file TEXT NOT NULL, package TEXT NOT NULL, version TEXT NOT NULL, path TEXT NOT NULL,
    leaf TEXT NOT NULL, kind TEXT NOT NULL, via TEXT NOT NULL, lines TEXT NOT NULL,
    role TEXT NOT NULL, workspace_package TEXT NOT NULL
);
CREATE INDEX usage_path ON usage (path);
CREATE INDEX usage_leaf ON usage (leaf COLLATE NOCASE);
CREATE INDEX usage_file ON usage (file);
CREATE INDEX usage_package ON usage (package, version);
CREATE TABLE file_state (file TEXT PRIMARY KEY, mtime_ns INTEGER NOT NULL, size INTEGER NOT NULL);
CREATE TABLE library_packages (lib TEXT NOT NULL, package TEXT NOT NULL, version TEXT);
"""

CATALOG_SCHEMA = """
CREATE TABLE libraries (lib TEXT PRIMARY KEY, record TEXT NOT NULL);
CREATE TABLE capabilities (id TEXT PRIMARY KEY, lib TEXT NOT NULL, record TEXT NOT NULL);
CREATE TABLE capability_items (id TEXT NOT NULL, item TEXT NOT NULL, leaf TEXT NOT NULL,
    spelling TEXT NOT NULL);
CREATE INDEX capability_items_item ON capability_items (item);
CREATE INDEX capability_items_leaf ON capability_items (leaf COLLATE NOCASE);
CREATE TABLE capability_files (id TEXT NOT NULL, path TEXT NOT NULL, role TEXT NOT NULL);
CREATE INDEX capability_files_path ON capability_files (path);
CREATE VIRTUAL TABLE capability_fts USING fts5(
    id UNINDEXED, lib, skill, feature, use_text, items, as_written, files,
    tokenize = "porter unicode61 tokenchars '_'"
);
"""


def _leaf(path: str) -> str:
    return path.removesuffix("!").rsplit("::", 1)[-1]


def ingest_records(conn: sqlite3.Connection, records: list[dict]) -> None:
    """Load the catalog records into the catalog tables and the full-text index."""
    skills = {r["lib"]: r.get("skill") or "" for r in records if r["kind"] == "library"}
    for record in records:
        blob = json.dumps(record, ensure_ascii=False)
        if record["kind"] == "library":
            conn.execute("INSERT INTO libraries VALUES (?, ?)", (record["lib"], blob))
            continue
        conn.execute(
            "INSERT INTO capabilities VALUES (?, ?, ?)", (record["id"], record["lib"], blob)
        )
        for spelling, key in (("item", "items"), ("as_written", "as_written")):
            for item in record.get(key, []):
                conn.execute(
                    "INSERT INTO capability_items VALUES (?, ?, ?, ?)",
                    (record["id"], item, _leaf(item), spelling),
                )
        files = [f["path"] for f in record.get("files", [])]
        wrapper = record.get("wrapper") or {}
        if wrapper.get("path") and wrapper["path"] not in files:
            files.append(wrapper["path"])
        for entry in record.get("files", []):
            conn.execute(
                "INSERT INTO capability_files VALUES (?, ?, ?)",
                (record["id"], entry["path"], entry["role"]),
            )
        if wrapper.get("path"):
            conn.execute(
                "INSERT INTO capability_files VALUES (?, ?, ?)",
                (record["id"], wrapper["path"], "wrapper-of"),
            )
        conn.execute(
            "INSERT INTO capability_fts VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                record["id"],
                record["lib"],
                skills.get(record["lib"], ""),
                record.get("feature", ""),
                record.get("use", ""),
                " ".join(record.get("items", [])),
                " ".join(record.get("as_written", [])),
                " ".join(files),
            ),
        )


def write_index(path: Path, run: Any, jsonl_text: str, roles: semantic.Roles) -> None:
    """Write the usage index for `run` (a `library_utilization.CatalogRun`), atomically."""
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temp = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.", suffix=".tmp")
    os.close(handle)
    try:
        conn = sqlite3.connect(temp)
        conn.executescript(INDEX_SCHEMA)
        rows = []
        for hit in run.hits:
            try:
                role = roles.role(hit.file, hit.lines)
            except OSError:
                role = ""
            rows.append(
                (
                    hit.file,
                    hit.package,
                    hit.version,
                    hit.path,
                    _leaf(hit.path),
                    hit.kind,
                    hit.via,
                    json.dumps(list(hit.lines)),
                    role,
                    roles.package(hit.file) or "",
                )
            )
        conn.executemany("INSERT INTO usage VALUES (?,?,?,?,?,?,?,?,?,?)", rows)
        files = scan.workspace_files(run.root, run.meta)
        state = []
        for rel in files:
            stat = (run.root / rel).stat()
            state.append((rel.as_posix(), stat.st_mtime_ns, stat.st_size))
        conn.executemany("INSERT INTO file_state VALUES (?,?,?)", state)
        conn.executemany("INSERT INTO library_packages VALUES (?,?,?)", run.packages)
        meta = {
            "schema_version": str(SCHEMA_VERSION),
            "run_stamp": run.stamp,
            "built_at": datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
            "jsonl_sha256": hashlib.sha256(jsonl_text.encode()).hexdigest(),
            "files_scanned": str(len(files)),
            "usage_rows": str(len(rows)),
            "blind_spots": json.dumps(list(BLIND_SPOTS)),
        }
        conn.executemany("INSERT INTO meta VALUES (?, ?)", meta.items())
        conn.commit()
        conn.close()
        os.replace(temp, path)
    except BaseException:
        Path(temp).unlink(missing_ok=True)
        raise


def _row(row: sqlite3.Row) -> dict[str, Any]:
    out = dict(row)
    if "lines" in out:
        out["lines"] = json.loads(out["lines"])
    return out


_TOKEN = re.compile(r"[A-Za-z0-9_]+")


def like_prefix(directory: str) -> str:
    """A LIKE pattern (ESCAPE '\\') matching everything under `directory`."""
    escaped = directory.rstrip("/") + "/"
    for char in ("\\", "%", "_"):
        escaped = escaped.replace(char, "\\" + char)
    return escaped + "%"


def fts_query(text: str) -> str:
    """An FTS5 expression matching any token of `text` as a prefix (recall first, rank orders)."""
    tokens = _TOKEN.findall(text.lower())
    return " OR ".join(f'"{t}"*' for t in tokens)


@dataclass(frozen=True)
class Fingerprint:
    """mtime and size of the two files a `Store` was built from, to notice a change."""

    jsonl: tuple[int, int]
    index: tuple[int, int] | None

    @staticmethod
    def of(jsonl: Path, index: Path) -> Fingerprint:
        def stat(p: Path) -> tuple[int, int] | None:
            try:
                s = p.stat()
            except FileNotFoundError:
                return None
            return (s.st_mtime_ns, s.st_size)

        return Fingerprint(stat(jsonl) or (0, 0), stat(index))


class Store:
    """The catalog (from the JSONL) plus the usage index (from the SQLite file), queryable."""

    def __init__(self, jsonl: Path, index: Path) -> None:
        self.jsonl, self.index_path = jsonl, index
        self.fingerprint = Fingerprint.of(jsonl, index)
        self.conn = sqlite3.connect(":memory:", uri=True, check_same_thread=False)
        self.conn.row_factory = sqlite3.Row
        self.conn.executescript(CATALOG_SCHEMA)
        self.conn.executescript(INDEX_SCHEMA)  # empty until an index is attached
        records = [json.loads(line) for line in jsonl.read_text().splitlines() if line.strip()]
        ingest_records(self.conn, records)
        self.has_usage = False
        self.meta: dict[str, str] = {}
        if index.exists():
            self._attach_index()

    def _attach_index(self) -> None:
        self.conn.execute("ATTACH DATABASE ? AS idx", (f"file:{self.index_path}?mode=ro",))
        for table in ("usage", "file_state", "library_packages", "meta"):
            self.conn.execute(f"INSERT INTO main.{table} SELECT * FROM idx.{table}")
        self.conn.commit()
        self.conn.execute("DETACH DATABASE idx")
        self.meta = {r["key"]: r["value"] for r in self.conn.execute("SELECT * FROM meta")}
        self.has_usage = True

    @property
    def stale(self) -> bool:
        """True when a file this store was built from has changed since (reload to refresh)."""
        return Fingerprint.of(self.jsonl, self.index_path) != self.fingerprint

    # -- catalog ---------------------------------------------------------------------------
    def libraries(self) -> list[dict]:
        return [
            json.loads(r["record"])
            for r in self.conn.execute("SELECT * FROM libraries ORDER BY lib")
        ]

    def library(self, lib: str) -> dict | None:
        row = self.conn.execute("SELECT record FROM libraries WHERE lib = ?", (lib,)).fetchone()
        return json.loads(row["record"]) if row else None

    def library_names(self) -> list[str]:
        return [r["lib"] for r in self.conn.execute("SELECT lib FROM libraries ORDER BY lib")]

    def capabilities(self, lib: str | None = None) -> list[dict]:
        sql, args = "SELECT record FROM capabilities", ()
        if lib is not None:
            sql, args = sql + " WHERE lib = ?", (lib,)
        return [json.loads(r["record"]) for r in self.conn.execute(sql + " ORDER BY id", args)]

    def search(self, query: str) -> list[tuple[dict, float]]:
        """Every capability matching any token of `query`, best first; empty matches all."""
        match = fts_query(query)
        if not match:
            return [(c, 0.0) for c in self.capabilities()]
        rows = self.conn.execute(
            "SELECT c.record AS record, bm25(capability_fts) AS score FROM capability_fts f "
            "JOIN capabilities c ON c.id = f.id WHERE capability_fts MATCH ? ORDER BY score",
            (match,),
        ).fetchall()
        return [(json.loads(r["record"]), r["score"]) for r in rows]

    def capabilities_with_items(self, items: list[str], leaf: str | None = None) -> list[dict]:
        """Capabilities listing any of `items` (item or `as_written`), or a leaf."""
        clauses, args = [], []
        for item in items:
            clauses.append("item = ?")
            args.append(item.removesuffix("!"))
            clauses.append("item = ?")
            args.append(item.removesuffix("!") + "!")
        if leaf:
            clauses.append("leaf = ? COLLATE NOCASE")
            args.append(leaf)
        if not clauses:
            return []
        ids = self.conn.execute(
            f"SELECT DISTINCT id FROM capability_items WHERE {' OR '.join(clauses)}", args
        ).fetchall()
        return self._capabilities_by_id([r["id"] for r in ids])

    def capabilities_for_path(self, path: str) -> list[dict]:
        """Capabilities whose files or wrapper are `path`, or under it (a directory)."""
        ids = self.conn.execute(
            "SELECT DISTINCT id FROM capability_files WHERE path = ? OR path LIKE ? ESCAPE '\\'",
            (path, like_prefix(path)),
        ).fetchall()
        return self._capabilities_by_id([r["id"] for r in ids])

    def _capabilities_by_id(self, ids: list[str]) -> list[dict]:
        if not ids:
            return []
        marks = ",".join("?" * len(ids))
        rows = self.conn.execute(
            f"SELECT record FROM capabilities WHERE id IN ({marks}) ORDER BY id", ids
        )
        return [json.loads(r["record"]) for r in rows]

    def skills(self) -> dict[str, list[str]]:
        found: dict[str, list[str]] = {}
        for lib in self.libraries():
            found.setdefault(lib.get("skill") or "", []).append(lib["lib"])
        return found

    # -- usage index -----------------------------------------------------------------------
    def usage_by_path(self, path: str) -> list[dict]:
        return self._usage("path = ?", (path.removesuffix("!"),))

    def usage_by_leaf(self, leaf: str) -> list[dict]:
        return self._usage("leaf = ? COLLATE NOCASE", (leaf.removesuffix("!"),))

    def usage_by_file(self, path: str) -> list[dict]:
        return self._usage("file = ? OR file LIKE ? ESCAPE '\\'", (path, like_prefix(path)))

    def usage_by_library(self, lib: str) -> list[dict]:
        return self._usage(
            "EXISTS (SELECT 1 FROM library_packages p WHERE p.lib = ? "
            "AND p.package = usage.package "
            "AND (p.version IS NULL OR p.version = usage.version))",
            (lib,),
        )

    def _usage(self, where: str, args: tuple) -> list[dict]:
        if not self.has_usage:
            return []
        rows = self.conn.execute(
            f"SELECT * FROM usage WHERE {where} ORDER BY path, file", args
        ).fetchall()
        return [_row(r) for r in rows]

    def file_states(self, files: list[str] | None = None) -> dict[str, tuple[int, int]]:
        if files is None:
            rows = self.conn.execute("SELECT * FROM file_state").fetchall()
        else:
            marks = ",".join("?" * len(files)) or "''"
            rows = self.conn.execute(
                f"SELECT * FROM file_state WHERE file IN ({marks})", files
            ).fetchall()
        return {r["file"]: (r["mtime_ns"], r["size"]) for r in rows}

    def usage_packages(self) -> list[dict]:
        if not self.has_usage:
            return []
        rows = self.conn.execute(
            "SELECT package, version, COUNT(*) AS rows, COUNT(DISTINCT file) AS files "
            "FROM usage GROUP BY package, version ORDER BY package, version"
        )
        return [dict(r) for r in rows]
