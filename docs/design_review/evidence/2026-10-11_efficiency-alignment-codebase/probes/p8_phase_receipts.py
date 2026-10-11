"""P8: aggregate `lctx_phase` elapsed_ms receipts from existing run logs (read-only, historical).

Run with `python3 -I probes/p8_phase_receipts.py RUNS_DIR`. For each run log containing phase
records, sums elapsed time per phase name (a phase can repeat, e.g. per stage, provider or batch)
and prints the run's largest phases plus selected groups:
  unrequested = analyze_local, evaluate_base, complete_base, prepare_source_calls,
                enrich_execution, apply_models, analyze_summaries, analyze_analytic,
                analytic_embedding
  catalog     = catalog_core, catalog_evidence, catalog_selection, synthesis, retrieval
These are historical receipts from fixture journeys and focused controls under mixed revisions,
profiles, fixtures and host load. Phases can nest, so sums are not additive to wall time. They
show which phases can dominate; they do not attribute cost to a mechanism.
"""
import re
import sys
from collections import defaultdict
from pathlib import Path

PHASE = re.compile(r'phase="([a-z_0-9]+)"[^\n]*?status="(\w+)" elapsed_ms=([0-9.]+)')
UNREQUESTED = {"analyze_local", "evaluate_base", "complete_base", "prepare_source_calls",
               "enrich_execution", "apply_models", "analyze_summaries", "analyze_analytic",
               "analytic_embedding"}
CATALOG = {"catalog_core", "catalog_evidence", "catalog_selection", "synthesis", "retrieval"}

runs = Path(sys.argv[1])
for log in sorted(runs.glob("*/output.log")):
    totals = defaultdict(float)
    counts = defaultdict(int)
    try:
        text = log.read_text(errors="replace")
    except OSError:
        continue
    for name, status, ms in PHASE.findall(text):
        totals[name] += float(ms) / 1000.0
        counts[name] += 1
    if len(totals) < 5:
        continue
    profile = "behavioral" if 'profile="behavioral"' in text else ("catalog" if 'profile="catalog"' in text else "?")
    top = sorted(totals.items(), key=lambda kv: kv[1], reverse=True)[:12]
    print(f"== {log.parent.name} profile={profile} phases={len(totals)}")
    print("   top: " + "; ".join(f"{k} {v:.1f}s x{counts[k]}" for k, v in top))
    print(f"   unrequested-group {sum(totals[k] for k in UNREQUESTED):.1f}s | catalog-group "
          f"{sum(totals[k] for k in CATALOG):.1f}s | artifact_manifest {totals.get('artifact_manifest', 0):.1f}s")
