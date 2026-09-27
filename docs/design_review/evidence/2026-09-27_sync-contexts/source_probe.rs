//! Observe the actual pinned providers at synchronous context sites; no source is executed.
fn main() {tokio::runtime::Runtime::new().unwrap().block_on(run());}
async fn run() {
    let dir=tempfile::tempdir().unwrap();
    let release=dir.path().join("release");let site=dir.path().join("venv/site-packages");
    std::fs::create_dir_all(&release).unwrap();std::fs::create_dir_all(&site).unwrap();
    std::fs::write(release.join("probe.py"),r#"
from contextlib import nullcontext, suppress

def plain(value):
    with nullcontext():
        return value

def bound(value):
    with nullcontext(value) as chosen:
        return chosen

def swallowed(value):
    with suppress(TypeError):
        raise 1
    return value

def nested(value):
    with nullcontext(), suppress(TypeError):
        return value

def opaque(value, manager):
    with manager:
        return value
"#).unwrap();
    let out=cpg_extract::extract(&cpg_extract::ExtractInput {
        release:cpg_extract::Release::from_tree(release,"sync-context-probe").unwrap(),
        venv_root:dir.path().join("venv"),site_packages:vec![site],python_version:(3,14,7),
        python_platform:"linux".into(),snapshot_id:cpg_schema::id::Id([9;16]),corpus:None,
        keep_pysa_json:false,test_hooks:Default::default(),
    }).unwrap();
    let ctx=cpg_core::snapshot::empty_session();
    for(name,batch)in out.tables {ctx.register_batch(name,batch).unwrap();}
    for query in [
        "SELECT module_name,qualified_name,kind,signature_count,key FROM context_definitions WHERE module_name='contextlib' ORDER BY qualified_name",
        "SELECT p.start_byte,p.end_byte,p.site_kind,p.phase,p.implicit_dunder_call,p.receiver_class,p.receiver_module,p.receiver_key,p.target_module,d.qualified_name,p.unresolved_reason FROM pysa_calls p LEFT JOIN context_definitions d ON d.module_name=p.target_module AND d.key=p.target_key AND d.kind=p.target_kind ORDER BY p.start_byte,p.phase",
        "SELECT r.name,rr.reason,rr.builtin_name,b.kind,b.start_byte,b.end_byte,x.resolved_module,x.imported_name,x.alias FROM references r JOIN reference_resolutions rr ON rr.reference_id=r.node_id LEFT JOIN bindings b ON b.node_id=rr.binding_id LEFT JOIN export_syntax x ON x.module_node_id=b.module_node_id AND x.start_byte=b.start_byte AND x.end_byte=b.end_byte ORDER BY r.start_byte",
        "SELECT symbol_node_id,signature_index,form,ordinal,kind,name,required FROM context_parameters WHERE symbol_node_id IN (SELECT symbol_node_id FROM context_definitions WHERE module_name='contextlib') ORDER BY symbol_node_id,signature_index,ordinal",
        "SELECT node_id,parent_node_id,kind,field,ordinal,detail,start_byte,end_byte FROM syntax_nodes WHERE kind IN (13,84) ORDER BY start_byte",
    ] {
        println!("{query}");
        let df=cpg_core::sql::query(&ctx,query).await.unwrap();
        println!("{}",datafusion::arrow::util::pretty::pretty_format_batches(&df.collect().await.unwrap()).unwrap());
    }
}
