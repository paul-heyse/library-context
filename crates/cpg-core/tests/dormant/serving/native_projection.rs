// DORMANT P5: supplied serving generations only; not a Cargo test target.
// Generation construction moves to typed P5 compilation. Removed Delta/import setup is excluded.
// These exact native/Python assertions remain recovery controls, not passing P4 receipts.
use std::path::Path;

pub fn run_native_script(generation_dir: &Path, script: &str) {
    let output = std::process::Command::new("uv")
        .args(["run", "--no-sync", "python", "-c", script])
        .arg(generation_dir)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output().unwrap();
    assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

/// Recovered from `analysis.rs`; the P5 fixture must retain its original source expectations.
pub fn typed_attributes_keep_source_evidence_through_serving(generation_dir: &Path) {
    run_native_script(generation_dir, r#"
import sys
from pathlib import Path
import pyarrow as pa
sys.path[:0] = [str(Path("scripts").resolve()), str(Path("python/lctx_mcp/tests").resolve())]
from support import load_native as load, served_bundle
from lctx_semantics import validate_projection_ipc
sys.path.insert(0, str(Path('python/lctx_mcp/tests').resolve()))
from test_projection import changed as changed_projection

generation = load(Path(sys.argv[1]), None)
with served_bundle(Path(sys.argv[1])) as pg:
    findings = pg.findings()
finding_ids = {r["finding_id"] for r in generation.tables["support_attribute_incidences"].to_pylist()}
assert finding_ids
for finding_id in finding_ids:
    result = findings[finding_id]
    assert result.attributes and result.attribute_incidences
    assert result.source_resolution in ('fact_only', 'source_span')
    assert all(row.source_fact_id and row.fact_table and row.fact_model_id for row in result.attribute_incidences)
rows = generation.tables['support_attribute_incidences'].to_pylist()
first = rows[0]
for mutation in ('missing', 'foreign', 'source_fact_id', 'fact_table', 'fact_model_id'):
    changed = dict(generation.tables)
    if mutation == 'missing':
        data = [r for r in rows if (r['finding_id'],r['object_node_id'],r['attribute_id']) !=
                (first['finding_id'],first['object_node_id'],first['attribute_id'])]
    elif mutation == 'foreign':
        data = [dict(r, object_node_id=bytes(16)) if r is first else r for r in rows]
    else:
        data = [dict(r, **{mutation: None}) if r is first else r for r in rows]
    changed['support_attribute_incidences'] = pa.Table.from_pylist(data, schema=generation.tables['support_attribute_incidences'].schema)
    try:
        validate_projection_ipc(*changed_projection(Path(sys.argv[1]), "support_attribute_incidences", changed["support_attribute_incidences"]))
    except ValueError:
        pass
    else:
        raise AssertionError((mutation, 'attribute support admitted'))
"#);
}

/// Recovered from `syntax.rs`; the P5 fixture must retain its original source expectations.
pub fn typed_handoff_pair_support_survives_serving_without_becoming_a_call_chain(generation_dir: &Path) {
    run_native_script(generation_dir, r#"
import sys
from pathlib import Path
import pyarrow as pa
sys.path[:0] = [str(Path("scripts").resolve()), str(Path("python/lctx_mcp/tests").resolve())]
from support import load_native as load, served_bundle
from lctx_semantics import validate_projection_ipc
sys.path.insert(0, str(Path('python/lctx_mcp/tests').resolve()))
from test_projection import changed as changed_projection

generation = load(Path(sys.argv[1]), None)
with served_bundle(Path(sys.argv[1])) as pg:
    findings = pg.findings()
handoffs = [key for key, row in findings.items() if row.kind == "handoff"]
assert handoffs
for key in handoffs:
    result = findings[key]
    assert not result.witnesses, 'a handoff pair is not a delegation chain'
    assert len(result.attributes) == 1
    attribute = result.attributes[0]
    assert attribute.kind in ('hands_off', 'takes_from')
    assert attribute.modality and attribute.phase and attribute.producer_modality and attribute.producer_phase
    assert result.attribute_incidences
    for row in result.attribute_incidences:
        assert row.object_node_id == result.subject_node_id
        assert row.site_node_id and row.edge_id and row.other_site_node_id and row.other_edge_id and row.consumer_formal_id
        assert row.source_fact_id and row.other_fact_id
    assert result.source_resolution == 'fact_only'
members = generation.tables['support_members'].to_pylist()
consumer = next(m for m in members if m['finding_id'] == handoffs[0] and m['role'] == 'consumer_site')
changed = dict(generation.tables)
changed['support_members'] = pa.Table.from_pylist([m for m in members if m is not consumer],
    schema=generation.tables['support_members'].schema)
try:
    validate_projection_ipc(*changed_projection(Path(sys.argv[1]), "support_members", changed["support_members"]))
except ValueError:
    pass
else:
    raise AssertionError('orphan handoff member admitted')
rows = generation.tables['support_attribute_incidences'].to_pylist()
for fields in (('other_fact_id',), ('other_edge_id', 'other_fact_id', 'other_fact_table', 'other_fact_model_id'),
               ('edge_id',), ('source_fact_id',), ('fact_table',), ('fact_model_id',)):
    changed = dict(generation.tables)
    changed['support_attribute_incidences'] = pa.Table.from_pylist(
        [dict(r, **{name: None for name in fields}) if r['finding_id'] == handoffs[0] else r for r in rows],
        schema=generation.tables['support_attribute_incidences'].schema)
    try:
        validate_projection_ipc(*changed_projection(Path(sys.argv[1]), "support_attribute_incidences", changed["support_attribute_incidences"]))
    except ValueError:
        pass
    else:
        raise AssertionError(('missing handoff evidence admitted', fields))
"#);
}

/// Recovered from `compile.rs`; the P5 fixture must retain its original source expectations.
pub fn transfer_alternatives_keep_condition_scopes_through_serving(generation_dir: &Path) {
    run_native_script(generation_dir, r#"
import sys
from pathlib import Path
sys.path[:0] = [str(Path("scripts").resolve()), str(Path("python/lctx_mcp/tests").resolve())]
from support import load_native as load, served_bundle
with served_bundle(Path(sys.argv[1])) as pg:
    generation = pg.load()
    mixed = pg.operation(generation, generation.snapshot_id, 'transferpkg.mixed')
    fates = [r.record for r in pg.section(generation.snapshot_id, mixed.operation_id, 'behavior') if r.record.parameter == 'value' and r.record.kind == 'returns']
    assert {f.transfer: f.verdict for f in fates} == {'identity': 'conditional', 'derived': 'conditional', 'call': 'unknown'}
    assert all(f.condition_scope_id == mixed.operation_id for f in fates)
    holder = pg.operation(generation, generation.snapshot_id, 'transferpkg.RecordHolder.__init__')
    fates = [r.record for r in pg.section(generation.snapshot_id, holder.operation_id, 'behavior') if r.record.parameter == 'value' and r.record.kind == 'returns']
    assert len(fates) == 3 and all(f.condition_scope_id != holder.operation_id for f in fates)
    assert {f.transfer for f in fates} == {'identity', 'derived', 'call'}
    assert all(f.verdict == 'unknown' for f in fates)
    unmodeled = pg.operation(generation, generation.snapshot_id, 'transferpkg.Holder.__init__')
    assert not [r.record for r in pg.section(generation.snapshot_id, unmodeled.operation_id, 'behavior') if r.record.parameter == 'value' and r.record.kind == 'returns']
"#);
}

/// Recovered from `compile.rs`; the P5 fixture must retain its original source expectations.
pub fn context_protocols_keep_native_ipc_certificates_and_value_identity(generation_dir: &Path) {
    run_native_script(generation_dir, r#"
import sys
from pathlib import Path
import pyarrow as pa
import pyarrow.ipc as ipc
from lctx_mcp.generation import NATIVE_IPC_FILES
sys.path[:0] = [str(Path("scripts").resolve()), str(Path("python/lctx_mcp/tests").resolve())]
from support import load_native as load, served_bundle
from lctx_semantics import SemanticExecutor
import json, subprocess
oracle=json.loads(subprocess.run([sys.executable,"docs/design_review/evidence/2026-09-27_sync-contexts/runtime_oracle.py"],capture_output=True,text=True,check=True,timeout=30).stdout)
observed={row["case"]:row for row in oracle["cases"]}
g=load(Path(sys.argv[1]),None)
for name in ("preserve","entry_value","suppress_type_error","multiple","assignment_failure_suppressed",
             "constructor_failure_suppressed","replacement_suppressed","return_preserved","matching_short_circuit"):
    paths,boundaries,total,truncated,work=g.condition_graph.inspect_value_paths("contextpkg."+name,"value","none","",True,0,20)
    assert observed[name]["outcome"]=={"kind":"return","value":42}
    assert observed[name]["identity_outcome"]=={"kind":"return","same_entry_value":True}
    assert paths and not truncated,(name,paths,boundaries)
    assert any(any(s[0]=="context_entry" for s in p[3]) for p in paths),(name,paths)
    assert any(any(s[0]=="context_exit" for s in p[3]) for p in paths),(name,paths)
for operation,formal in (("entry_value","value"),("entry_keyword","value"),("entry_other","other")):
    paths,_,_,truncated,_=g.condition_graph.inspect_value_paths("contextpkg."+operation,formal,"none","",True,0,20)
    assert paths and not truncated,(operation,paths)
    assert all(any(s[0]=="context_entry_value_identity" for s in p[3]) for p in paths)
    assert all(not any(s[0]=="raw_identity" for s in p[3]) for p in paths)
paths,*_=g.condition_graph.inspect_value_paths("contextpkg.entry_other","value","none","",True,0,20)
assert not paths,paths
for mutation in ("missing_site","missing_certificate","duplicate_certificate","missing_match",
                 "wrong_condition","missing_group","same_function_sites", "missing_context_value", "duplicate_context_value",
                 "omitted_value_basis", "foreign_context_value"):

    sites_by_function={}
    for row in g.tables["source_context_sites"].to_pylist():
        sites_by_function.setdefault(row["function_node_id"],[]).append(row["site_id"])
    twins=next(sites for sites in sites_by_function.values() if len(sites)>1)
    swapped={twins[0]:twins[1],twins[1]:twins[0]}
    files=[]
    for name in NATIVE_IPC_FILES:
        table=g.tables[name]
        rows=table.to_pylist()
        if name=="source_context_sites" and mutation=="missing_site":
            rows=rows[1:]
        if name=="return_completion_certificates" and mutation=="missing_certificate":
            rows=rows[1:]
        if name=="return_completion_certificates" and mutation=="duplicate_certificate":
            rows.append(rows[0].copy())
        if name=="source_context_value_identities":
            if mutation in ("missing_context_value","omitted_value_basis"): rows=[]
            if mutation=="duplicate_context_value": rows.append(rows[0].copy())
            if mutation=="foreign_context_value": rows[0]["context_site_id"]=twins[0]
        if name=="summary_flow_steps":
            if mutation=="omitted_value_basis":
                rows=[r for r in rows if r["kind"]!="context_entry_value_identity"]
            if mutation=="same_function_sites":
                for row in rows:
                    if row["kind"] in ("context_construction","context_entry","context_exit"):
                        row["evidence_id"]=swapped.get(row["evidence_id"],row["evidence_id"])
            if mutation=="missing_match":
                rows=[r for r in rows if r["kind"]!="context_exception_evidence"]
            if mutation=="missing_group":
                rows=[r for r in rows if not r["kind"].startswith("context_")]
            if mutation=="wrong_condition":
                for row in rows:
                    if row["kind"]=="context_entry": row["condition_id"]=b"\xff"*16
            counters={}
            for row in rows:
                key=row["summary_id"];row["ordinal"]=counters.get(key,0);counters[key]=row["ordinal"]+1
        altered=pa.Table.from_pylist(rows,schema=table.schema)
        sink=pa.BufferOutputStream()
        with ipc.new_file(sink,table.schema) as writer: writer.write_table(altered)
        files.append((name,sink.getvalue().to_pybytes()))
    try:
        SemanticExecutor.from_ipc(1,g.snapshot_id,g.manifest["entry_value_effect_digest"],files)
    except ValueError:
        pass
    else:
        raise AssertionError("accepted "+mutation)
"#);
}

/// Fresh source calls retain body/binding/release evidence in the native serving executor.
/// The referenced independent Python probe remains the contract; no fixture body is executed here.
pub fn fresh_source_calls_require_body_binding_and_release(generation_dir: &Path) {
    let output = std::process::Command::new("uv")
        .args(["run", "--no-sync", "python", "docs/design_review/evidence/2026-09-27_frame-exit/native_source_call.py"])
        .arg(generation_dir)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
