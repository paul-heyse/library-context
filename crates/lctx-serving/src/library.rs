//! Exact first-party capture admission and collection metadata, resolved per request.
use lctx_model::domain::{ModelError, serving::{LibraryAdmissionData,PreparedLibraryDomains,Name,LibraryDomainPacket},resources::ResourceBudget};
use lctx_surrealdb::NativeReader;
use surrealdb::types::{Variables,RecordId};
pub async fn resolve(reader:&NativeReader,name:Option<&Name>,budget:&ResourceBudget)->Result<Vec<LibraryDomainPacket>,ModelError> {
    let mut vars=Variables::new();vars.insert("name",name.map(|n|n.as_str().to_owned()));
    let roots:Vec<RecordId>=reader.query("RETURN fn::lctx_library_roots($name);",vars).await?;
    let batches=crate::scope::hydrate(reader,roots,&LibraryAdmissionData::inputs(),budget).await?;
    let mut data=LibraryAdmissionData::new(budget);
    for (name,batch) in &batches.batches {data.visit(name,batch)?;}
    let prepared=PreparedLibraryDomains::prepare(&data,budget)?;
    let resolved=prepared.resolve(name).map_err(|e|ModelError::Invalid(e.to_string()))?;
    Ok(resolved.metadata(budget)?.domains)
}
pub fn native_definitions()-> &'static str {
    r#"
DEFINE FUNCTION fn::lctx_library_roots($name: option<string|null>) {
 LET $packages=SELECT VALUE id FROM entity WHERE semantic_type='packages' AND ($name=NONE OR $name=NULL OR body.name=$name);
 LET $releases=SELECT VALUE in FROM reference WHERE field='package' AND out IN $packages;
 LET $all_distributions=SELECT VALUE in FROM participant WHERE field='release' AND out IN $releases;
 LET $distributions=SELECT VALUE id FROM assertion WHERE id IN $all_distributions AND semantic_type='input_distributions' AND body.role=0;
 LET $inputs=SELECT VALUE out FROM participant WHERE in IN $distributions AND field='input';
 LET $corpora=SELECT VALUE in FROM participant WHERE out IN $inputs AND field='library' AND in.semantic_type='corpus_libraries';
 LET $corpus_inputs=SELECT VALUE out FROM participant WHERE in IN $corpora AND field='corpus';
 LET $all_inputs=array::distinct(array::concat($inputs,$corpus_inputs));
 LET $runs=SELECT VALUE in FROM reference WHERE out IN $all_inputs AND field='input' AND in.semantic_type='provider_runs';
 LET $sources=SELECT VALUE in FROM reference WHERE out IN $all_inputs AND field='input' AND in.semantic_type='source_artifacts';
 LET $modules=SELECT VALUE in FROM reference WHERE out IN $sources AND field='source' AND in.semantic_type='modules';
 LET $scope_targets=array::distinct(array::concat($all_inputs,$releases,$sources,$modules));
 LET $scopes=SELECT VALUE in FROM reference WHERE out IN $scope_targets AND field IN ['input','release','artifact','module'] AND in.semantic_type='coverage_scopes';
 LET $coverage=array::distinct(array::concat((SELECT VALUE in FROM participant WHERE out IN $runs AND field='run' AND in.semantic_type='provider_coverage'),(SELECT VALUE in FROM participant WHERE out IN $scopes AND field='scope' AND in.semantic_type='provider_coverage')));
 RETURN array::distinct(array::concat($distributions,$inputs,$corpora,$corpus_inputs,$runs,$coverage));
};
"#
}
