"""Hybrid retrieval over one generation (DESIGN §11.2).

1. Lexical: BM25 (bm25s, numpy backend, `get_scores`) over `lexical_text`, with our tokenizer,
   over the query's **discriminating** words only: a word every brief contains cannot tell them
   apart, and a ranking by it alone is BM25's length normalization. With none, the lexical leg
   abstains (ADR-0010 amendment, increment-1 deep review F2).
2. Vector: exact cosine against the instruction-prefixed query vector; a brief's score is its best
   chunk's.
3. Fusion: reciprocal-rank fusion, K = 60, 1-based ranks, ties by brief id.
4. Exact symbols: a query that is a public access path promotes its briefs, recorded as promoted.
A score ranks briefs; it is never evidence that a brief fits the task.
"""

from __future__ import annotations

import re

import bm25s
import numpy as np
import pyarrow as pa
import pyarrow.ipc as ipc

from lctx_mcp.wire import Contract

K = 60
TOKEN = re.compile(r"[a-z0-9]+")


def tokenize(text: str) -> list[str]:
    """Lower-cased runs of letters and digits: `custom_route` is `custom route`."""
    return TOKEN.findall(text.lower())


class Lexical:
    """A BM25 index of the briefs' lexical texts, in brief order."""

    def __init__(self, texts: list[str]) -> None:
        self.retriever = bm25s.BM25(k1=1.5, b=0.75, method="lucene", backend="numpy")
        tokens = [tokenize(t) for t in texts]
        self.retriever.index(tokens, show_progress=False)
        self.size = len(texts)
        self.df: dict[str, int] = {}
        for doc in tokens:
            for t in set(doc):
                self.df[t] = self.df.get(t, 0) + 1

    def discriminating(self, query: str) -> list[str]:
        """The query's words some briefs contain and others do not, in query order."""
        return [t for t in tokenize(query) if 0 < self.df.get(t, 0) < self.size]

    def scores(self, query: str, *, discriminating: bool = True) -> np.ndarray:
        """Score matching words; brief ranking may abstain on fully shared vocabulary."""
        tokens = (
            self.discriminating(query)
            if discriminating
            else [t for t in tokenize(query) if self.df.get(t, 0)]
        )
        if not tokens:
            return np.zeros(self.size, dtype=np.float32)
        return np.asarray(self.retriever.get_scores(tokens), dtype=np.float32)


FUSION_BYTES = 128 * 1024 * 1024
RANK_BYTES = 32 * 1024 * 1024


RetrievalMetadata = Contract("RetrievalMetadata")


def identities(values: list[str]) -> np.ndarray:
    return np.frombuffer(b"".join(bytes.fromhex(value) for value in values), dtype="V16")


def lexical_rank(ids: np.ndarray, scores: np.ndarray, eligible: np.ndarray) -> np.ndarray:
    keep = (scores > 0) & np.isin(ids, eligible, assume_unique=True)
    ids, scores = ids[keep], scores[keep]
    return ids[np.lexsort((ids, -scores))]


def vector_legs(raw: bytes, schema_ipc: bytes) -> list[np.ndarray]:
    if len(raw) > RANK_BYTES:
        raise ValueError("resource_refused: rank IPC byte budget")
    table = ipc.open_stream(pa.BufferReader(raw)).read_all()
    expected = ipc.open_stream(pa.BufferReader(schema_ipc)).schema
    if table.schema != expected:
        raise ValueError("corrupt: rank schema")
    ids = np.frombuffer(b"".join(table.column("entity_id").to_pylist()), dtype="V16")
    views = table.column("view").to_numpy()
    ranks = table.column("rank").to_numpy()
    result = []
    for view in sorted(set(views)):
        selected = np.flatnonzero(views == view)
        selected = selected[np.argsort(ranks[selected])]
        if not np.array_equal(ranks[selected], np.arange(1, len(selected) + 1)):
            raise ValueError("corrupt: non-contiguous vector ranks")
        result.append(ids[selected])
    return result


def fuse_legs(
    legs: list[np.ndarray], promoted: np.ndarray, limit: int
) -> list[tuple[str, float, str, bool]]:
    """Compact RRF over complete exact ranks or explicitly labelled candidate ranks.

    The conservative ledger covers all arrays and sorting temporaries simultaneously. It is
    checked before concatenation; no full-universe Python score/rank dictionaries are allocated.
    """
    pairs = sum(len(leg) for leg in legs) + len(promoted)
    if pairs * 160 > FUSION_BYTES:
        raise ValueError("resource_refused: aggregate fusion byte budget")
    if not pairs:
        return []
    universe = np.unique(np.concatenate([*legs, promoted]))
    scores = np.zeros(len(universe), dtype=np.float64)
    lexical_seen = np.zeros(len(universe), dtype=np.bool_)
    vector_seen = np.zeros(len(universe), dtype=np.bool_)
    for i, leg in enumerate(legs):
        indices = np.searchsorted(universe, leg)
        scores[indices] += 1.0 / (K + np.arange(1, len(leg) + 1, dtype=np.float64))
        (lexical_seen if i == 0 else vector_seen)[indices] = True
    is_promoted = np.isin(universe, promoted, assume_unique=True)
    order = np.lexsort((universe, -scores, ~is_promoted))[:limit]
    out = []
    for i in order:
        source = (
            "exact_symbol"
            if is_promoted[i]
            else "hybrid"
            if lexical_seen[i] and vector_seen[i]
            else "lexical"
            if lexical_seen[i]
            else "vector"
        )
        out.append(
            (universe[i].tobytes().hex(), round(float(scores[i]), 6), source, bool(is_promoted[i]))
        )
    return out


class UnitLexical:
    """Index unique family/text inputs; retain all addressable member occurrences."""

    def __init__(self, fragments: pa.Table, subjects: pa.Table) -> None:
        members: dict[bytes, set[str]] = {}
        for row in subjects.to_pylist():
            if row["member_id"] is not None:
                members.setdefault(row["unit_id"], set()).add(row["member_id"].hex())
        unique: dict[tuple[str, str], list[tuple[str, str, set[str]]]] = {}
        for row in fragments.to_pylist():
            unique.setdefault((row["family"], row["text"]), []).append(
                (row["unit_id"].hex(), row["fragment_id"].hex(), members.get(row["unit_id"], set()))
            )
        self.families = {}
        for family in sorted({key[0] for key in unique}):
            keys = sorted(key for key in unique if key[0] == family)
            self.families[family] = (
                Lexical([key[1] for key in keys]),
                [unique[key] for key in keys],
            )

    def unit_winners(self, query: str, families: list[str]) -> list[dict]:
        """One vote per actual unit, including release-only or unassociated originals."""
        best = {}
        for family in families:
            if family not in self.families:
                continue
            index, occurrences = self.families[family]
            scores = index.scores(query, discriminating=False)
            for position in np.flatnonzero(scores > 0):
                for unit, fragment, _members in occurrences[int(position)]:
                    row = {
                        "family": family,
                        "channel": "lexical",
                        "unit_id": unit,
                        "fragment_id": fragment,
                        "score": float(scores[position]),
                        "rank": 0,
                    }
                    old = best.get(unit)
                    if old is None or (-row["score"], fragment) < (
                        -old["score"],
                        old["fragment_id"],
                    ):
                        best[unit] = row
                    if len(best) * 512 > FUSION_BYTES:
                        raise ValueError("resource_refused: lexical unit budget")
        return list(best.values())

    def winners(self, query: str, eligible: set[str]) -> list[dict]:
        best: dict[tuple[str, str], dict] = {}
        work = 0
        for family, (index, occurrences) in self.families.items():
            scores = index.scores(query)
            for position in np.flatnonzero(scores > 0):
                for unit, fragment, members in occurrences[int(position)]:
                    for member in members & eligible:
                        work += 1
                        if work * 512 > FUSION_BYTES:
                            raise ValueError("resource_refused: lexical occurrence budget")
                        row = {
                            "member_id": member,
                            "family": family,
                            "channel": "lexical",
                            "unit_id": unit,
                            "fragment_id": fragment,
                            "score": float(scores[position]),
                            "rank": 0,
                        }
                        key = (member, family)
                        old = best.get(key)
                        if old is None or (-row["score"], unit, fragment) < (
                            -old["score"],
                            old["unit_id"],
                            old["fragment_id"],
                        ):
                            best[key] = row
        result = sorted(best.values(), key=lambda r: (r["family"], -r["score"], r["member_id"]))
        counts: dict[str, int] = {}
        for row in result:
            counts[row["family"]] = counts.get(row["family"], 0) + 1
            row["rank"] = counts[row["family"]]
        return result


def unit_vector_rows(raw: bytes, schema_ipc: bytes) -> list[dict]:
    if len(raw) > RANK_BYTES:
        raise ValueError("resource_refused: unit rank IPC budget")
    table = ipc.open_stream(pa.BufferReader(raw)).read_all()
    expected = ipc.open_stream(pa.BufferReader(schema_ipc)).schema
    if table.schema != expected:
        raise ValueError("corrupt: addressable rank schema")
    rows = table.to_pylist()
    counts: dict[str, int] = {}
    for row in rows:
        for key in ("member_id", "unit_id", "fragment_id"):
            row[key] = row[key].hex()
        counts[row["family"]] = counts.get(row["family"], 0) + 1
        if (
            row["rank"] != counts[row["family"]]
            or row["channel"] != "vector"
            or not np.isfinite(row["score"])
        ):
            raise ValueError("corrupt: unit vector ranks")
    return rows


def fuse_families(rows: list[dict], promoted: set[str]) -> list[dict]:
    """RRF within families, then equal-weight family ranks; exact paths sort first."""
    if (len(rows) + len(promoted)) * 1024 > FUSION_BYTES:
        raise ValueError("resource_refused: addressable fusion budget")
    members = sorted({row["member_id"] for row in rows} | promoted)
    if not members:
        return []
    positions = {member: i for i, member in enumerate(members)}
    totals = np.zeros(len(members), dtype=np.float64)
    witnesses: dict[str, list[dict]] = {member: [] for member in members}
    seen = set()
    for row in rows:
        key = (row["member_id"], row["family"], row["channel"])
        if key in seen or row["rank"] < 1:
            raise ValueError("corrupt: duplicate member family channel")
        seen.add(key)
        witnesses[row["member_id"]].append(row)
    for family in sorted({row["family"] for row in rows}):
        selected = [row for row in rows if row["family"] == family]
        indices = np.asarray([positions[row["member_id"]] for row in selected])
        ranks = np.asarray([row["rank"] for row in selected], dtype=np.float64)
        contributions = np.bincount(indices, weights=1.0 / (K + ranks), minlength=len(members))
        available = np.flatnonzero(contributions > 0)
        order = available[np.lexsort((available, -contributions[available]))]
        totals[order] += 1.0 / (K + np.arange(1, len(order) + 1, dtype=np.float64))
    exact = np.asarray([member in promoted for member in members])
    order = np.lexsort((np.arange(len(members)), -totals, ~exact))
    return [
        {
            "member_id": members[i],
            "score": float(totals[i]),
            "promoted": bool(exact[i]),
            "winners": sorted(witnesses[members[i]], key=lambda r: (r["family"], r["channel"])),
        }
        for i in order
    ]
