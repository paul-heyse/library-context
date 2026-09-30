"""Summarize retained pilot outputs, layer equality, normalized scopes and stored topology."""

import json
import re
from pathlib import Path

from pilots import CASES, HERE, RAW, sql


def main() -> None:
    receipt = {}
    for label, (frontier, profile) in CASES.items():
        published = json.loads((RAW / f"{label}.stdout").read_text())
        relations = json.loads((RAW / f"{label}-relations.json").read_text())
        errors = (RAW / f"{label}.stderr").read_text()
        peak = re.search(r"Maximum resident set size \(kbytes\): (\d+)", errors)
        assert peak
        families = {r["family"]: r["availability"] for r in published["families"]}
        assert (families["Flow"] == "NotRequested") == (profile == "catalog")
        generation = published["generation"]
        assert re.fullmatch(r"[0-9a-f]{32}", generation)
        measured = {
            "generation": generation,
            "content_digest": published["content_digest"],
            "stage_measurements": published["stage_measurements"],
            "peak_reservation_bytes": published["peak_reservation_bytes"],
            "time_v_maximum_rss_bytes": int(peak.group(1)) * 1024,
            "relation_storage_bytes": sum(r["storage_bytes"] for r in relations),
            "relation_rows": sum(r["row_count"] for r in relations),
            "families": families,
        }
        if frontier == "normalized":
            for name, query in {
                "snapshots": f"SELECT a.projection,a.vertices,a.arcs,a.gaps,a.availability,s.bytes,s.chunks,s.codec,s.petgraph_version FROM lctx_g{generation}.projection_source_assessments a JOIN lctx_g{generation}.projection_snapshots s ON s.assessment=a.id ORDER BY a.id",
                "capability_availability": f"SELECT n.capability,n.availability AS aggregate,c.availability,count(*) AS scopes FROM lctx_g{generation}.normalization_computations n LEFT JOIN lctx_g{generation}.normalization_coverage c ON c.computation=n.id GROUP BY n.capability,n.availability,c.availability ORDER BY n.capability,c.availability",
            }.items():
                measured[name] = json.loads(sql(f"SELECT coalesce(json_agg(t),'[]'::json) FROM ({query}) t"))
            facts = json.loads((RAW / f"facts-{profile}-relations.json").read_text())
            current = {r["relation_name"]: r for r in relations}
            assert all(current[r["relation_name"]]["content_digest"] == r["content_digest"] for r in facts)
            measured["cumulative_facts_content_equality"] = "passed"
            counts = {r["relation_name"]: r["row_count"] for r in relations}
            measured["join_cardinalities"] = {name: counts.get(name) for name in ["provider_symbols", "symbol_declarations", "symbol_entity_resolutions", "symbol_entity_candidates", "provider_call_sites", "call_targets", "normalized_call_events", "normalized_call_alternatives", "call_binding_attempts", "call_bindings", "normalization_coverage", "normalization_coverage_premises", "normalization_evidence_members"]}
        receipt[label] = measured
    assert receipt["normalized-behavioral"]["content_digest"] == receipt["normalized-behavioral-repeat"]["content_digest"]
    receipt["conditions"] = "Separate fresh processes, pinned acquired inputs, shared host, warm build and filesystem caches; no cache flush or controlled speedup experiment. Stage reservations include charged collections/replay; transient native state, SQLx buffers and allocator retention remain external RSS allowances."
    (HERE / "measurement-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print("passed: five profile receipts, layer content equality, repeated content and stored topology")


if __name__ == "__main__":
    main()
