#!/usr/bin/env bash
# Reproduces the synthetic probes of this folder. Scratch state lives under
# build/review-probes/graph-analytics-parity/ (gitignored). Containers are removed at the end.
# Passwords are local throwaway values for disposable containers.
# Optional: run_all.sh <generation schema>  also runs the real-data probes against that facts
# layer (read-only export; Neo4j capped at 6 GiB, SurrealDB at 6 GiB for the real graph).
set -euo pipefail
ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
HERE="$ROOT/docs/design_review/evidence/2026-10-05_graph-analytics-parity"
WORK="$ROOT/build/review-probes/graph-analytics-parity"
PW="${GAP_PROBE_PASSWORD:-$(python3 -c 'import secrets; print(secrets.token_hex(16))')}"  # disposable, per run
SCHEMA=${1:-}
mkdir -p "$WORK" "$HERE/raw"

if [ ! -x "$WORK/.venv/bin/python" ]; then
  uv venv -q -p 3.14 "$WORK/.venv"
  uv pip install -q -p "$WORK/.venv/bin/python" neo4j==6.3.1 networkx requests numpy scipy
fi
PY="$WORK/.venv/bin/python"

cleanup() { docker rm -f -v gap-neo4j gap-surreal >/dev/null 2>&1 || true; }
trap cleanup EXIT
cleanup
docker run -d --name gap-neo4j -p 127.0.0.1:17687:7687 -e NEO4J_AUTH=neo4j/$PW \
  -e 'NEO4J_PLUGINS=["graph-data-science","apoc"]' \
  -e NEO4J_server_memory_heap_max__size=2G -e NEO4J_server_memory_heap_initial__size=1G \
  neo4j:2026.09.0-community-trixie >/dev/null
# memory-capped: unbounded +path / {..} on a cyclic graph exhausts memory (see README)
docker run -d --name gap-surreal --memory=4g --memory-swap=4g -p 127.0.0.1:18000:8000 \
  surrealdb/surrealdb:v3.3.0 start --user root --pass $PW memory >/dev/null
until docker logs gap-neo4j 2>&1 | grep -q 'Started.'; do sleep 2; done

for mode in modular uniform; do
  "$PY" "$HERE/gds_determinism.py" bolt://127.0.0.1:17687 $PW "$HERE/raw/gds_determinism_$mode.json" "$mode" >/dev/null
done
"$PY" "$HERE/surreal_traversal.py" http://127.0.0.1:18000 $PW "$HERE/raw/surreal_traversal.json" >/dev/null
"$PY" "$HERE/surreal_g4_diagnose.py" http://127.0.0.1:18000 $PW "$HERE/raw/surreal_g4_diagnose.json" >/dev/null

SMOKE="$WORK/neo4rs-smoke"
mkdir -p "$SMOKE/src"
cp "$HERE/neo4rs_smoke.Cargo.toml" "$SMOKE/Cargo.toml"
cp "$HERE/neo4rs_smoke.rs" "$SMOKE/src/main.rs"
(cd "$SMOKE" && CARGO_BUILD_BUILD_DIR="$SMOKE/build-dir" CARGO_TARGET_DIR="$SMOKE/target" \
  CARGO_BUILD_JOBS=4 RUSTC_WRAPPER= cargo build -q)
NEO4J_PROBE_PASSWORD=$PW "$SMOKE/target/debug/neo4rs-smoke" 127.0.0.1:17687 > "$HERE/raw/neo4rs_smoke_bolt4.txt"

if [ -n "$SCHEMA" ]; then
  FG="$WORK/facts-graph"
  "$HERE/export_facts_graph.sh" "$SCHEMA" "$FG"
  cp "$FG/summary.csv" "$HERE/raw/facts_graph_summary.csv"
  REF="$WORK/petgraph-ref"
  mkdir -p "$REF/src"
  cp "$HERE/petgraph_ref.Cargo.toml" "$REF/Cargo.toml"
  cp "$HERE/petgraph_ref.rs" "$REF/src/main.rs"
  (cd "$REF" && CARGO_BUILD_BUILD_DIR="$REF/build-dir" CARGO_TARGET_DIR="$REF/target" \
    CARGO_BUILD_JOBS=4 RUSTC_WRAPPER= cargo build -q --release)
  "$REF/target/release/petgraph-ref" "$FG" >/dev/null
  cp "$FG/ref_summary.json" "$HERE/raw/petgraph_ref_summary.json"
  cleanup
  docker run -d --name gap-neo4j --memory=6g --memory-swap=6g -p 127.0.0.1:17687:7687 -e NEO4J_AUTH=neo4j/$PW \
    -e 'NEO4J_PLUGINS=["graph-data-science","apoc"]' -e NEO4J_server_memory_heap_max__size=3G \
    -e NEO4J_server_memory_heap_initial__size=2G -e NEO4J_server_memory_pagecache_size=1G \
    neo4j:2026.09.0-community-trixie >/dev/null
  docker run -d --name gap-surreal --memory=6g --memory-swap=6g -p 127.0.0.1:18000:8000 \
    surrealdb/surrealdb:v3.3.0 start --user root --pass $PW memory >/dev/null
  until docker logs gap-neo4j 2>&1 | grep -q 'Started.'; do sleep 2; done
  "$PY" "$HERE/gds_real.py" bolt://127.0.0.1:17687 $PW "$FG" "$HERE/raw/gds_real.json" >/dev/null
  "$PY" "$HERE/surreal_real.py" http://127.0.0.1:18000 $PW "$FG" "$HERE/raw/surreal_real.json" >/dev/null
fi
