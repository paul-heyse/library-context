"""Manual quantifier-scope classification of every InvariantCheck/PublicationCheck impl in
crates/lctx-model/src/domain (read 2026-10-05 at f66eb15a). Run from the repository root:
    python3 docs/design_review/evidence/2026-10-05_snapshot-diff/classify_checks.py
It re-enumerates impls so a new or renamed check shows up as UNCLASSIFIED.
Classes:
 R  row-local / forward reference with a streaming probe: targets loaded into a keyed map, each
    referrer row checked on visit (FK existence, subtype, bound against a target attribute);
    finish is empty or trivial.
 K  key-local group: a parent row carries a digest/count of its children; children arrive ordered
    by parent key and are flushed per parent; finish only asserts no parent left without children.
 F  multi-hop forward reference/structure resolved in finish over in-memory indexes (each row's
    referenced closure must exist and agree); per-row, but the closure crosses relations; includes
    per-group cardinality and transitive parent chains.
 G  global / closed-world: reverse completeness (no unreferenced rows), whole-graph property
    (acyclic), or cross-relation count equality.
 D  derivation replay: finish recomputes the owning stage's output from its whole declared inputs
    and requires exact equality (output == f(inputs)); closed-world over the stage's outputs.
"""
import re, pathlib, collections, sys
ROOT = pathlib.Path("crates/lctx-model/src/domain")
C = {
 # R: streaming forward reference / row predicate
 **{k: "R" for k in ["source.rs:OccurrenceBounds", "assertion.rs:EvidenceCheck", "assertion.rs:QualificationCheck",
   "attribution.rs:CoverageOwnership", "calls.rs:ModuleOwners", "calls.rs:CallerCheck", "calls.rs:TargetCheck",
   "calls.rs:NativeSupportCheck", "declarations.rs:DeclarationCheck", "deployment.rs:ReportShape",
   "flow.rs:ReachingCheck", "flow.rs:FlowStructure", "input.rs:OwnershipCheck", "input.rs:AcquisitionBoundaries",
   "lexical.rs:LexicalCheck", "syntax.rs:SyntaxGeometry", "transfer.rs:FrameCheck", "types/generics.rs:GenericCheck",
   "types.rs:BodyCheck", "input.rs:ClassCheck", "types/signatures.rs:PortCheck"]},
 # K: parent-carries-digest groups
 **{k: "K" for k in ["artifact.rs:ArtifactContents", "calls.rs:OriginCheck", "calls.rs:CallSyntaxCheck",
   "deployment.rs:CollectionCheck", "flow_capture.rs:InventoryCheck", "input.rs:InputManifestCheck",
   "symbols.rs:SequenceCheck", "types.rs:ParameterListCheck", "types.rs:DictFieldCheck", "types.rs:SequenceCheck",
   "value.rs:SetCheck", "calls.rs:ResolutionCheck", "flow.rs:PathCheck", "types.rs:RecordCheck",
   "class_metadata.rs:TransformCheck", "attribution.rs:InvocationCheck"]},
 # F: multi-hop forward closure in finish
 **{k: "F" for k in ["calls.rs:SignatureCheck", "calls/signature_enumeration.rs:EnumerationCheck",
   "symbols.rs:ExportCheck", "symbols.rs:SymbolCheck", "assumptions.rs:Check", "assumptions_universe.rs:Check",
   "flow.rs:ViewCheck", "types/locations.rs:Check", "types/overload_origins.rs:Check", "types.rs:TypeIndex",
   "protocols.rs:Check", "syntax.rs:BoundaryCheck", "documents.rs:DocumentCheck", "conditions/rebase.rs:GuardCheck",
   "types/queries.rs:Check", "analysis/native.rs:InventoryCheck", "execution/completion_records.rs:CompletionCheck",
   "execution/summary_proof.rs:CostCheck", "analysis/family/invocation.rs:InvocationCheck",
   "analysis/family/support.rs:SupportCheck"]},
 # G: reverse completeness / whole-graph / count equality
 **{k: "G" for k in ["conditions/mod.rs:CatalogCheck", "derivation.rs:Check", "flow_inventory.rs:InventoryCheck",
   "analysis/family/source_receipts.rs:SourcePublicationCheck",
   "execution/completion_production.rs:CompletionProfileCheck", "execution/enriched_production.rs:ProfileCheck",
   "execution/model_production.rs:ProfileCheck", "execution/production.rs:EvaluationProfileCheck",
   "execution/source_call_records.rs:ProfileCheck", "execution/summary_replay.rs:ProfileCheck",
   "structural/frames.rs:ProfileCheck"]},
}
D_FILES = ("normalized/", "catalog/", "retrieval/", "selection/", "synthesis/", "structural/frames.rs:Check",
           "analytics/frames.rs", "projection/", "embedding/", "execution/", "local_", "atom_decision.rs",
           "conditions/entry.rs", "conditions/stability.rs", "composition.rs", "models/records.rs",
           "analysis/family/coverage.rs", "analysis/family/obligations.rs", "analysis/findings.rs",
           "analysis/frontier.rs")
def block(t, s):
    i = t.index("{", s); d = 0
    for j in range(i, len(t)):
        d += {"{": 1, "}": -1}.get(t[j], 0)
        if d == 0: return t[i:j + 1]
rows = []
for f in sorted(ROOT.rglob("*.rs")):
    t = f.read_text(); rel = str(f.relative_to(ROOT))
    for m in re.finditer(r"impl(?:<[^>]*>)?\s+(?:super::)*(?:\w+::)*(InvariantCheck|PublicationCheck) for (\w+)", t):
        key = f"{rel}:{m.group(2)}"
        cls = C.get(key) or ("D" if any(key.startswith(p) or p in key for p in D_FILES) else "UNCLASSIFIED")
        rows.append((key, m.group(1), cls))
print("check\ttrait\tclass")
for r in rows: print("\t".join(r))
cnt = collections.Counter(r[2] for r in rows)
print("# totals " + " ".join(f"{k}={v}" for k, v in sorted(cnt.items())) + f" all={len(rows)}", file=sys.stderr)
