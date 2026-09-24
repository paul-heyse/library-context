"""The gold matcher, version 2 (DESIGN §12; pre-registered in ADR-0010's 2026-09-24 amendment).

Identity is the declaration node. A gold operation resolves by exact equality of its path with a
served `public_paths` row (own and inherited spellings, exported classes included), with no fuzzy
or suffix fallback, so a family's node set is its resolved operations' nodes. A class operation
resolves to the class's node, so it matches only a brief seeded by that class. An operation that
resolves to nothing is listed apart, under the release's public root or outside it, and stays in
the family's Jaccard union as a string.

Evaluation only: nothing here feeds the compiler (§1.4).
"""

from __future__ import annotations

from dataclasses import dataclass

MATCHER_VERSION = 2


@dataclass(frozen=True)
class Family:
    """A gold family as the matcher sees it."""

    id: str
    nodes: frozenset[bytes]
    unresolved_under_root: tuple[str, ...]
    outside_root: tuple[str, ...]

    @property
    def size(self) -> int:
        """|F|: the resolved nodes and the unresolved operations' strings."""
        return len(self.nodes) + len(self.unresolved_under_root) + len(self.outside_root)


def paths(public_paths: list[dict]) -> dict[str, bytes]:
    """Each served public path's node: the one lookup every script resolves by (R2 F2)."""
    return {r["access_path"]: r["node_id"] for r in public_paths}


def roots(public_paths: list[dict]) -> set[str]:
    """The release's public roots, as the served paths show them (not the distribution name)."""
    return {r["access_path"].split(".")[0] for r in public_paths}


def class_nodes(public_paths: list[dict]) -> set[bytes]:
    """The nodes of exported classes."""
    return {r["node_id"] for r in public_paths if r["kind"] == "class"}


def status(embedder_name: str, degraded_aliases: int) -> str:
    """`blocked` when live vectors were asked for and any alias answered without them."""
    return "blocked" if embedder_name == "vllm" and degraded_aliases else "measured"


def resolve(families: list[dict], public_paths: list[dict]) -> list[Family]:
    """Each family's operations resolved by exact path against the served public paths; an
    operation that resolves to nothing is under a public root or outside every one."""
    by_path = paths(public_paths)
    under_roots = roots(public_paths)
    out = []
    for f in families:
        nodes: set[bytes] = set()
        under: list[str] = []
        outside: list[str] = []
        for op in f["operations"]:
            node = by_path.get(op)
            if node is not None:
                nodes.add(node)
            elif op.split(".")[0] in under_roots:
                under.append(op)
            else:
                outside.append(op)
        out.append(Family(f["id"], frozenset(nodes), tuple(sorted(under)), tuple(sorted(outside))))
    return out


def jaccard(family: Family, seed: bytes) -> float:
    """(a): the size of F intersect {seed} over the size of F union {seed}, a brief contributing
    its seed node."""
    inside = seed in family.nodes
    union = family.size + (0 if inside else 1)
    return (1.0 if inside else 0.0) / union if union else 0.0


def hits(family: Family, seed: bytes) -> bool:
    """(b): a returned brief is a hit when its seed is in the family's node set."""
    return seed in family.nodes


def node_of(public_paths: list[dict], path: str) -> bytes | None:
    """The node a public path names, or None (the same lookup as `resolve`)."""
    return paths(public_paths).get(path)
