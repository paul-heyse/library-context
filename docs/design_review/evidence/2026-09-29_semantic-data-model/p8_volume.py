"""P8: derivation/support volume versus total and graph-catalog volume in a published store.

Read-only. Uses only the `snapshots` publication rows (per-table row counts) and on-disk sizes.
Usage: python3 p8_volume.py <store-dir>
"""
import glob
import os
import sys

import pyarrow.parquet as pq

store = sys.argv[1] if len(sys.argv) > 1 else "build/store"
rows = []
for f in glob.glob(os.path.join(store, "snapshots", "*.parquet")):
    rows += pq.read_table(f).to_pylist()
snapshots = sorted({r["snapshot_id"].hex() for r in rows})
counts = {r["table_name"]: r["row_count"] for r in rows}
support = {k: v for k, v in counts.items()
           if k.endswith("_steps") or k in ("witnesses", "finding_members", "behavior_discharges")}
graph = {k: counts.get(k, 0) for k in ("nodes", "edges")}


def mib(table):
    total = 0
    for root, _, files in os.walk(os.path.join(os.path.realpath(store), table)):
        total += sum(os.path.getsize(os.path.join(root, f)) for f in files)
    return total / 2**20


print(f"store={os.path.realpath(store)} snapshots={snapshots} tables={len(counts)}")
print(f"total_rows={sum(counts.values())}")
print(f"support_rows={sum(support.values())} {dict(sorted(support.items()))}")
print(f"graph_catalog_rows={sum(graph.values())} {graph}")
print("summary_rows", {k: counts.get(k) for k in (
    "summary_components", "summary_flows", "summary_flow_steps", "summary_boundaries",
    "summary_origin_coverage", "value_flows", "behaviors", "behavior_discharges")})
tables = [d for d in os.listdir(os.path.realpath(store)) if d in counts]
sizes = {t: mib(t) for t in tables}
print(f"canonical_table_MiB={sum(sizes.values()):.1f} nodes+edges_MiB={sizes.get('nodes', 0) + sizes.get('edges', 0):.1f} "
      f"support_MiB={sum(sizes.get(t, 0) for t in support):.1f}")
