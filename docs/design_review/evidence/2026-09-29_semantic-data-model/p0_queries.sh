#!/usr/bin/env bash
# P0: compile the known-answer fixture and read what the existing relations state per shape.
# Usage (repository root): docs/design_review/evidence/2026-09-29_semantic-data-model/p0_queries.sh <scratch-dir>
set -euo pipefail
E=docs/design_review/evidence/2026-09-29_semantic-data-model
W=${1:?scratch directory}
rm -rf "$W/p0-store" "$W/p0-gen"
snap=$(./target/release/lctx compile-fixture --package p0shapes --profile behavioral --seed p0shapes.fetch \
  --store "$W/p0-store" --generations "$W/p0-gen" "$E/p0-fixture" | awk '/^snapshot/{print $2}')
q() { ./target/release/lctx query --store "$W/p0-store" --snapshot "$snap" "$1"; }
echo "snapshot $snap"
echo "## summary_flows"
q "SELECT d.qualified_name fn, s.input_path, s.output_path, s.kind, s.verdict, s.path_depth FROM summary_flows s JOIN declarations d ON d.node_id = s.function_node_id ORDER BY fn, input_path"
echo "## summary_boundaries vs summary_origin_coverage (same origin)"
q "SELECT d.qualified_name fn, p.name param, b.reason boundary_reason, c.reason coverage_reason FROM summary_boundaries b JOIN declarations d ON d.node_id=b.function_node_id LEFT JOIN parameter_syntax p ON p.node_id=b.parameter_node_id LEFT JOIN summary_origin_coverage c ON c.subject_id=b.source_origin_id AND c.function_node_id=b.function_node_id ORDER BY fn"
echo "## value_flows (intra-procedural)"
q "SELECT d.qualified_name fn, v.source_name src, v.sink, v.place, v.identity id, v.through_call thru, v.condition, a.keyword, a.ordinal FROM value_flows v JOIN declarations d ON d.node_id=v.function_node_id LEFT JOIN arguments a ON a.node_id=v.argument_node_id ORDER BY fn, src, v.sink_start_byte"
echo "## behaviors (served in the behavioral profile)"
q "SELECT d.qualified_name op, b.kind, b.transfer, b.parameter_name p, b.callee_text, b.target_name tgt, b.value, b.verdict, b.boundary_reason br, b.condition FROM behaviors b JOIN declarations d ON d.node_id=b.operation_node_id ORDER BY op, b.kind, p"
