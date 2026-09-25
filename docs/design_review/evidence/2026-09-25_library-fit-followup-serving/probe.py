"""Read-only serving contract probe over the existing analysis_shapes generation."""

from pathlib import Path

from lctx_mcp import operations as ops
from lctx_mcp.generation import load
from lctx_mcp.server import hydrate, markdown, serve

root = Path("build/py-fixture")
gen = load(root / (root / "CURRENT").read_text().strip(), None)
print(
    "PROVENANCE",
    {
        "generation": gen.key,
        "snapshot": gen.snapshot_id,
        "format": gen.manifest["format"],
        "compiler_digest": gen.manifest["compiler_digest"],
        "fresh_compile": False,
    },
)
for (facet, value), rows in gen.by_facet.items():
    if rows and all(v not in ops.MATCHING for v in rows.values()):
        node = next(iter(rows))
        op = ops.get_operation(gen, gen.snapshot_id, node.hex())
        found = ops.find_operations(
            gen,
            ops.Where(facets=[ops.FacetTerm(facet=facet, value=value)]),
            50,
            None,
        )
        print(
            "FACET_CASE",
            {
                "operation": op.access_path,
                "facet": facet,
                "value": value,
                "stored_verdict": rows[node],
                "get_operation_values": op.facets.get(facet),
                "find_total": found.total,
                "unknown_total": found.unknown_total,
            },
        )
        break
else:
    raise RuntimeError("the existing fixture has no unknown-only facet case")

served = serve(gen, None)
for bid in gen.brief_ids:
    capability = hydrate(served, gen.snapshot_id, bid.hex())
    claim = next(
        (
            a
            for a in capability.assertions
            if a.kind == "coordinates"
            and a.supports
            and all(s.evidence_id is None for s in a.supports)
        ),
        None,
    )
    if claim:
        print(
            "CLAIM_CASE",
            {
                "path": capability.access_path,
                "text": claim.text,
                "supports": [s.model_dump() for s in claim.supports],
                "evidence_count": len(capability.evidence),
                "finding_id_in_markdown": claim.supports[0].finding_id
                in markdown(capability),
                "bundle_has_findings": "findings" in gen.tables,
            },
        )
        break
else:
    raise RuntimeError("the existing fixture has no finding-only coordinates assertion")
