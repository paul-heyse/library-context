"""Challenge a freshly compiled source-call generation through the production native reader."""

import sys
from pathlib import Path

import pyarrow as pa
import pyarrow.ipc as ipc
from lctx_semantics import SemanticExecutor, kernel_format

from lctx_mcp.generation import NATIVE_IPC_FILES, load


def main() -> None:
    generation = load(Path(sys.argv[1]), None)
    index = generation.condition_graph
    assert index is not None
    checked = 0
    for name in ("call_literal", "call_fallthrough", "call_return_finally", "call_modeled"):
        paths, _, _, truncated, _ = index.inspect_value_paths(
            "bodypkg." + name, "value", "none", "", True, 0, 20,
        )
        assert paths and not truncated, name
        assert any(any(step[0] == "source_call_normal" for step in p[3]) for p in paths), name
        checked += 1
    for name in ("call_raises", "call_local_read", "call_default", "call_captured",
                 "call_intervening", "call_alias", "call_unreachable"):
        paths, _, _, truncated, _ = index.inspect_value_paths(
            "bodypkg." + name, "value", "none", "", True, 0, 20,
        )
        assert not paths and not truncated, name
        checked += 1

    def altered(changes: dict[str, list[dict]]) -> None:
        files = []
        for name in NATIVE_IPC_FILES:
            table = generation.tables[name]
            if name in changes:
                table = pa.Table.from_pylist(changes[name], schema=table.schema)
            sink = pa.BufferOutputStream()
            with ipc.new_file(sink, table.schema) as writer:
                writer.write_table(table)
            files.append((name, sink.getvalue().to_pybytes()))
        SemanticExecutor.from_ipc(kernel_format(), generation.snapshot_id,
                                  generation.manifest["entry_value_effect_digest"], files)

    changes = [{name: []} for name in ("source_call_normals", "source_call_header_steps",
                                      "source_body_completions", "source_body_steps",
                                      "source_body_release_inputs", "model_frame_exits")]
    rows = generation.tables["summary_flow_steps"].to_pylist()
    selected = next(r["summary_id"] for r in rows if r["kind"] == "source_call_normal")
    group = sorted((r for r in rows if r["summary_id"] == selected), key=lambda r: r["ordinal"])
    at = next(i for i, r in enumerate(group) if r["kind"] == "source_call_normal")
    for start in (at, at - 2):
        kept = [dict(r) for i, r in enumerate(group) if not start <= i <= at]
        for ordinal, row in enumerate(kept):
            row["ordinal"] = ordinal
        other = [r for r in rows if r["summary_id"] != selected]
        changes.append({"summary_flow_steps": other + kept})
    # Use a separately valid certificate, not a fabricated hash, at this exact occurrence.
    certificates = generation.tables["source_call_normals"].to_pylist()
    current = next(r for r in certificates if r["certificate_id"] == group[at]["evidence_id"])
    foreign = next(r for r in certificates
                   if r["function_node_id"] != current["function_node_id"])
    substituted = [dict(r) for r in group]
    substituted[at]["evidence_id"] = foreign["certificate_id"]
    changes.append({"summary_flow_steps": [r for r in rows if r["summary_id"] != selected]
                    + substituted})
    for change in changes:
        try:
            altered(change)
        except ValueError:
            checked += 1
        else:
            raise AssertionError(("missing semantic support accepted", next(iter(change))))
    print(f"passed: {checked} native source-call controls")


if __name__ == "__main__":
    main()
