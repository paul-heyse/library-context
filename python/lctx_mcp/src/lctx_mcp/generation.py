"""Pinned PostgreSQL identity with only native and lexical state resident in Python."""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pyarrow as pa
import pyarrow.ipc as ipc
from lctx_semantics import SemanticExecutor, native_files
from lctx_storage import PinnedRepository

from lctx_mcp.embedder import Spec

FORMAT = 13
KERNEL_FORMAT = 1
_CONDITION_FILES, _SURFACE_FILES, _SUMMARY_FILES, _LIMITS = native_files()
NATIVE_IPC_FILES = (
    frozenset(_CONDITION_FILES) | frozenset(_SURFACE_FILES) | frozenset(_SUMMARY_FILES)
)


class GenerationError(RuntimeError):
    """A generation this server must not serve; startup fails."""


@dataclass(frozen=True)
class LexicalState:
    brief_ids: np.ndarray
    brief_text: list[str]
    operation_ids: np.ndarray
    operation_text: list[str]


@dataclass(frozen=True)
class Generation:
    repository: PinnedRepository
    descriptor: dict
    lexical_state: LexicalState
    condition_graph: SemanticExecutor | None

    @property
    def manifest(self) -> dict:
        return self.descriptor["manifest"]

    @property
    def key(self) -> str:
        return self.descriptor["generation"]

    @property
    def snapshot_id(self) -> str:
        return self.manifest["snapshot_id"]

    @property
    def library(self) -> str:
        return self.manifest["context"]["library"]

    @property
    def spec_hash(self) -> str | None:
        return self.manifest["spec_hash"]

    @property
    def summary(self) -> dict:
        return self.manifest["context"]["summary"]


def load(
    repository: PinnedRepository,
    descriptor: dict,
    inputs: dict[str, bytes],
    client_spec: Spec | None,
) -> Generation:
    """Build only native and lexical consumers of Rust-verified immutable artifacts."""
    manifest = descriptor["manifest"]
    if manifest["bundle_format"] != FORMAT:
        raise GenerationError("incompatible bundle format")
    specs = ipc.open_file(pa.BufferReader(inputs["embedding_spec"])).read_all()
    if manifest["spec_hash"] is not None:
        if specs.num_rows != 1:
            raise GenerationError("embedding spec missing")
        spec = Spec.from_json(specs.column("spec")[0].as_py())
        if spec.hash != manifest["spec_hash"] or (
            client_spec is not None and spec.hash != client_spec.hash
        ):
            raise GenerationError("query embedding spec differs from the pinned generation")
    native = (
        SemanticExecutor.from_ipc(
            KERNEL_FORMAT,
            manifest["snapshot_id"],
            manifest["entry_value_effect_digest"],
            [(name, inputs[name]) for name in sorted(NATIVE_IPC_FILES)],
        )
        if manifest["capabilities"]["native_value_paths"]
        else None
    )

    def lexical(name: str, key: str) -> tuple[np.ndarray, list[str]]:
        table = ipc.open_file(pa.BufferReader(inputs[name])).read_all()
        ids = np.frombuffer(b"".join(table.column(key).to_pylist()), dtype="V16").copy()
        return ids, table.column("text").to_pylist()

    briefs, brief_text = lexical("lexical_text", "brief_id")
    operations, operation_text = lexical("operation_text", "node_id")
    return Generation(
        repository, descriptor, LexicalState(briefs, brief_text, operations, operation_text), native
    )
