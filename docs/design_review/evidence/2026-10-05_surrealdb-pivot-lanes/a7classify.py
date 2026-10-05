import json,re
from collections import Counter,defaultdict
exec(open('/tmp/claude-1000/-home-paul-library-context/3307aa74-dd84-437f-93be-1d92a29d0cce/scratchpad/a7load.py').read())
fm=json.load(open('/tmp/claude-1000/-home-paul-library-context/3307aa74-dd84-437f-93be-1d92a29d0cce/scratchpad/a7fm.json'))
def module(n): return fm[n][0].split('/')[0].replace('.rs','') if fm[n] else 'generated-owner-family'
LIFE_GEN=('analysis_coverage_premises','analysis_inputs','analysis_invocations','analysis_outcomes','coverage_required_sources','coverage_requirements','coverage_sources','invocation_sources','projection_inputs','source_receipts')
LIFE_SFX=('receipts','admissions','coverage','outcomes','invocations','requirements','boundaries','witnesses','premises','inputs','assessments','attempts','obligations','derivations','completions','runs','refusals','frontiers','standings','costs','residuals','proofs','discharge_evidence','sources','computations','characterizations','restrictions','releases')
VALUE_SFX=('values','options','defaults','shapes','sets','lists','sequences','definitions','configurations','specifications','templates','channels','projections','origins')
BLOBS={'artifact_chunks','projection_snapshot_chunks','analysis_embedding_uses','retrieval_embedding_uses','analytic_text_windows','retrieval_corpus_texts','retrieval_fragments','synthesis_brief_documents'}
ATTR_EXPLICIT={'assertion_qualifications','evidence','provider_surfaces','run_families'}
def arms(r):
    if 'sum' not in r: return []
    return [set(x['name'] for x in a['fields']) for a in r['sum']['arms']]
def eff_refs(r, drop_qual=True):
    A=arms(r); inarm=set().union(*A) if A else set()
    def sem(f): return f['type']=='id' and not f['provenance'] and not (drop_qual and f.get('target')=='assertion_qualifications')
    base=[f for f in r['fields'] if sem(f) and f['name'] not in inarm]
    mx=max([sum(1 for f in r['fields'] if f['name'] in a and sem(f)) for a in A],default=0)
    return len(base)+mx, base
def is_ref_union(r):
    A=arms(r)
    if not A: return False
    inarm=set().union(*A)
    if any(f['type']=='id' and f['name'] not in inarm for f in r['fields']): return False
    return all(sum(1 for f in r['fields'] if f['name'] in a and f['type']=='id')<=1 for a in A)
def classify(n):
    r=R[n]; mod=module(n); sfx=n.split('_')[-1]
    if n in BLOBS: return 7,'blob/chunk/text-or-vector body'
    if mod=='generated-owner-family' and n.endswith(LIFE_GEN): return 6,'generated owner lifecycle family'
    if n.endswith('_supports') or n in ATTR_EXPLICIT: return 4,'support/attribution'
    if n.endswith('_observations') and r['layer']=='L0': return 4,'provider observation'
    if mod in ('analytics','embedding','retrieval','projection','structural') :
        if n.endswith(VALUE_SFX) and mod!='structural': return 5,'analytics config/definition'
        return 8,'analytics/projection/derived ('+mod+')'
    if n.endswith(LIFE_SFX) or n.endswith('_evidence'): return 6,'lifecycle/proof ('+sfx+')'
    if n.endswith(VALUE_SFX) or n.startswith('literal_') or not any(f['type']=='id' for f in r['fields']):
        return 5,'vocabulary/value/collection header'
    if is_ref_union(r): return 5,'tagged reference union (sum arms)'
    e,_=eff_refs(r)
    if e<=1: return 1,'entity/node'
    if e==2: return 2,'binary'
    return 3,'n-ary'
C={n:classify(n) for n in R}
# --- refinement: qualifier refs vs endpoint refs; composite-identity nodes
QUAL_T={'assertion_qualifications','analysis_contexts','coverage_scopes','evidence','provider_runs','provider_surfaces','providers','input_revisions','assumption_sets','conditions','provider_coverage','analytics_configurations','native_analysis_premises'}
def is_qual_target(t):
    return t in QUAL_T or t.endswith(('_analysis_invocations','_analysis_coverage','_analysis_outcomes','_source_receipts','_runs'))
def endpoint_refs(r):
    A=arms(r); inarm=set().union(*A) if A else set()
    ep=lambda f: f['type']=='id' and not f['provenance'] and not is_qual_target(f['target'])
    base=[f for f in r['fields'] if ep(f) and f['name'] not in inarm]
    mx=max([sum(1 for f in r['fields'] if f['name'] in a and ep(f)) for a in A],default=0)
    return len(base)+mx
NODES={'places','access_paths','provider_symbols','provider_modules','provider_callables','type_variables','signature_parameters','signature_slots','signature_variants','normalized_call_events','evaluation_atoms','condition_nodes','type_terms','parameter_entities','catalog_members','catalog_callables','catalog_classes','catalog_constructors','flow_uses','flow_definitions','binding_events','lexical_scopes','derived_artifacts','callable_entities','class_entities','field_entities','document_nodes','place_roots','predicates','providers','packages','input_revisions','analysis_contexts','report_collections','releases','modules','occurrences','source_artifacts','retrieval_units','synthesis_briefs','catalog_scenarios','summary_components','local_type_domains','local_field_locations','base_field_locations'}
def classify2(n):
    c,why=classify(n)
    if n in NODES: return 1,'entity/node (composite identity)'
    if c in (1,2,3):
        e=endpoint_refs(R[n])
        if e<=1: return 1,'entity/node'
        if e==2: return 2,'binary'
        return 3,'n-ary'
    return c,why
C={n:classify2(n) for n in R}
for n in list(C):
    if n.endswith('_transfer_alternatives'): C[n]=(4,'qualification wrapper over transfer key')
NAMES={1:'entity/node',2:'binary relationship',3:'n-ary/hyperedge',4:'observation/support/attribution',5:'vocabulary/value/tagged-union',6:'lifecycle/proof/receipt/coverage',7:'artifact/blob chunk',8:'projection/analytics/derived',9:'other'}
