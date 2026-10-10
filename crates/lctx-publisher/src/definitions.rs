//! Immutable named executable epochs. Other publications may install additional names
//! without changing this publication's realization identity.
use lctx_model::domain::{ContentHash, Key, KeySink, ModelError};
use lctx_surrealdb::{Loader, surrealdb::types::{Object, Value, ToSql}};
use surrealdb_sql::{Expr, TopLevelExpr, statements::{define::DefineKind, DefineStatement}};
use std::collections::BTreeSet;

pub(crate) fn epoch_identity(blueprint:&str)->ContentHash {
    let mut key=KeySink::new("native-definition-epoch/v1");
    key.part(b"blueprint",blueprint.as_bytes());
    key.part(b"view-relative-library",LIBRARY_BODY.as_bytes());
    key.part(b"vector-policy",b"exact-eligible-cosine/v1");
    key.finish()
}
pub(crate) fn function_name(epoch:ContentHash,name:&str)->String {format!("lctx_e{}_{}",epoch.hex(),name.strip_prefix("lctx_").unwrap_or(name))}

// The selected payloads and nominal targets have deliberately distinct identities.
const LIBRARY_BODY:&str=r#"{
 LET $selected=SELECT VALUE node FROM compiler_view_member WHERE view IN $views;
 LET $packages=SELECT VALUE id FROM entity WHERE id IN $selected AND semantic_type='packages' AND ($name=NONE OR $name=NULL OR body.name=$name);
 LET $releases=SELECT VALUE in FROM reference WHERE in IN $selected AND field='package' AND out IN $packages.anchor;
 LET $distributions=SELECT VALUE in FROM participant WHERE in IN $selected AND field='release' AND out IN $releases.anchor AND in.semantic_type='input_distributions' AND in.body.role=0;
 LET $input_anchors=SELECT VALUE out FROM participant WHERE in IN $distributions AND field='input';
 LET $inputs=SELECT VALUE node FROM compiler_view_member WHERE view IN $views AND node.anchor IN $input_anchors;
 LET $corpora=SELECT VALUE in FROM participant WHERE in IN $selected AND field='library' AND out IN $inputs.anchor AND in.semantic_type='corpus_libraries';
 LET $corpus_anchors=SELECT VALUE out FROM participant WHERE in IN $corpora AND field='corpus';
 LET $corpus_inputs=SELECT VALUE node FROM compiler_view_member WHERE view IN $views AND node.anchor IN $corpus_anchors;
 LET $all_inputs=array::distinct(array::concat($inputs,$corpus_inputs));
 LET $runs=SELECT VALUE in FROM reference WHERE in IN $selected AND out IN $all_inputs.anchor AND field='input' AND in.semantic_type='provider_runs';
 LET $sources=SELECT VALUE in FROM reference WHERE in IN $selected AND out IN $all_inputs.anchor AND field='input' AND in.semantic_type='source_artifacts';
 LET $modules=SELECT VALUE in FROM reference WHERE in IN $selected AND out IN $sources.anchor AND field='source' AND in.semantic_type='modules';
 LET $scope_targets=array::distinct(array::concat($all_inputs,$releases,$sources,$modules));
 LET $scopes=SELECT VALUE in FROM reference WHERE in IN $selected AND out IN $scope_targets.anchor AND field IN ['input','release','artifact','module'] AND in.semantic_type='coverage_scopes';
 LET $coverage=array::distinct(array::concat((SELECT VALUE in FROM participant WHERE in IN $selected AND out IN $runs.anchor AND field='run' AND in.semantic_type='provider_coverage'),(SELECT VALUE in FROM participant WHERE in IN $selected AND out IN $scopes.anchor AND field='scope' AND in.semantic_type='provider_coverage')));
 RETURN array::distinct(array::concat($distributions,$inputs,$corpora,$corpus_inputs,$runs,$coverage));
}"#;
fn expected(blueprint:&str)->Result<(Vec<String>,Vec<String>),ModelError>{
    lctx_surrealdb::materialization::validate_native_definitions(blueprint)?;
    let epoch=epoch_identity(blueprint);
    let mut base=Vec::new();let mut functions=Vec::new();
    for statement in surrealdb_syn::parse(blueprint).map_err(ModelError::codec)?.expressions {
        match statement {
            TopLevelExpr::Expr(Expr::Define(mut definition))=>match definition.as_mut(){
                DefineStatement::Function(function)=>{
                    function.name=function_name(epoch,function.name.as_str()).into();
                    function.kind=DefineKind::Default;
                    let sql=if function.name.as_str().ends_with("_library_roots") {
                        format!("DEFINE FUNCTION fn::{}($name: option<string|null>, $views: array<record<compiler_view>>) {LIBRARY_BODY} PERMISSIONS FULL",function.name)
                    }else{definition.to_sql()};
                    functions.push(normalize(&sql)?);
                },
                _=>base.push(normalize(&definition.to_sql())?),
            },
            _=>return Err(ModelError::Schema("publication definition blueprint grammar")),
        }
    }
    Ok((base,functions))
}
fn normalize(sql:&str)->Result<String,ModelError>{
    let parsed=surrealdb_syn::parse(sql).map_err(ModelError::codec)?;
    if parsed.expressions.len()!=1{return Err(ModelError::Schema("single definition inventory"));}
    Ok(parsed.expressions[0].to_sql())
}
async fn info(loader:&Loader,sql:String)->Result<Object,ModelError>{
    let mut response=loader.client().query(sql).await.map_err(ModelError::codec)?.check().map_err(ModelError::codec)?;
    let value:Value=response.take(0).map_err(ModelError::codec)?;
    let Value::Object(object)=value else{return Err(ModelError::Schema("definition inventory object"));};Ok(object)
}
fn add_group(inventory:&mut BTreeSet<String>,object:&Object,group:&str)->Result<(),ModelError>{
    let Some(Value::Object(definitions))=object.get(group) else{return Err(ModelError::Schema("definition inventory group"));};
    for value in definitions.values(){let Value::String(sql)=value else{return Err(ModelError::Schema("definition inventory text"));};inventory.insert(normalize(sql)?);}
    Ok(())
}
async fn inventory(loader:&Loader)->Result<BTreeSet<String>,ModelError>{
    let db=info(loader,"INFO FOR DB".into()).await?;
    let mut result=BTreeSet::new();for group in ["functions","analyzers","tables"]{add_group(&mut result,&db,group)?;}
    let Some(Value::Object(tables))=db.get("tables") else{return Err(ModelError::Schema("definition table inventory"));};
    for name in tables.keys(){let escaped=name.replace('`',"\\`");let table=info(loader,format!("INFO FOR TABLE `{escaped}`")).await?;for group in ["fields","indexes"]{add_group(&mut result,&table,group)?;}}
    Ok(result)
}
pub(crate) async fn install_epoch(loader:&Loader,blueprint:&str)->Result<ContentHash,ModelError>{
    let (base,functions)=expected(blueprint)?;let actual=inventory(loader).await?;
    if base.iter().any(|definition|!actual.contains(definition)){return Err(ModelError::Conflict("installed native search/schema definitions"));}
    for definition in &functions {
        if actual.contains(definition){continue;}
        // DEFINE's default refuses an existing conflicting immutable name. The readback
        // independently compares its actual body; IF NOT EXISTS is never evidence of equality.
        let created=loader.client().query(definition).await.and_then(|response|response.check());
        if let Err(error)=created {
            if !inventory(loader).await?.contains(definition){return Err(lctx_surrealdb::loader::write_failure(error));}
        }
    }
    verify_epoch(loader,blueprint).await
}
pub(crate) async fn verify_epoch(loader:&Loader,blueprint:&str)->Result<ContentHash,ModelError>{
    let (base,functions)=expected(blueprint)?;let actual=inventory(loader).await?;
    let mut key=KeySink::new("native-view-realization/v1");
    let version=loader.client().version().await.map_err(ModelError::codec)?.to_string();
    if !version.starts_with("3.3."){return Err(ModelError::Conflict("reviewed native engine family"));}
    key.part(b"engine",version.as_bytes());
    epoch_identity(blueprint).encode(&mut key);
    lctx_surrealdb::schema::realization_identity(blueprint).encode(&mut key);
    for definition in base.iter().chain(functions.iter()) {
        if !actual.contains(definition){return Err(ModelError::Conflict("pinned executable definition epoch"));}
        key.part(b"actual-definition",definition.as_bytes());
    }
    Ok(key.finish())
}
pub(crate) use verify_epoch as verify_realization;
