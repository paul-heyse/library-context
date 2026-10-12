//! Explicit end of a request's warm retention. This registers retirement; it never
//! advances a collector or releases another consumer's hold.
use super::*;
use lctx_model::domain::{completion::{Completion,complete},resources::ResourceBudget};

/// Release precisely this request/source-attempt product after its producer has ended.
/// None means the exact product is absent, not evidence that another owner was released.
/// A repeated successful call returns the original durable retirement invocation.
pub async fn release_retained_product(client:Arc<Surreal<Client>>,request:ContentHash,
    source_attempt:ContentHash,budget:&ResourceBudget)->Result<Option<ContentHash>,ModelError>{
    let reader=crate::NativeReader::private(client.clone()).with_budget(budget);
    let result=async {
        let product=RecordId::new("native_product",format!("{}_{}",request.hex(),source_attempt.hex()));
        let Some(expected)=held::inspect_expected_read(&reader,product.clone(),budget).await? else{return Ok(None)};
        let row=expected.row().as_object().ok_or(ModelError::Schema("retained product release row"))?;
        if row.get("request")!=Some(&Value::String(request.hex())) {
            return Err(ModelError::Conflict("retained product release exact request"));
        }
        if row.get("released")==Some(&Value::Bool(true)) {
            let Some(Value::RecordId(job))=row.get("release_job") else{return Err(ModelError::Schema("released product nomination"))};
            if job.table.as_str()!="native_retirement" {return Err(ModelError::Schema("released product nomination family"))}
            return Ok(Some(record_hash(job)?));
        }
        if row.get("released")!=Some(&Value::Bool(false)) || row.get("release_job").is_some_and(|value|*value!=Value::None) {
            return Err(ModelError::Conflict("retained product release state"));
        }
        let Some(Value::RecordId(contribution))=row.get("contribution") else{return Err(ModelError::Schema("retained product release contributor"))};
        let mut source_query=Variables::new();source_query.insert("release_source",RecordId::new("native_attempt",source_attempt.hex()));
        let sources:Vec<Object>=reader.query_native("SELECT * FROM $release_source",source_query).await?;
        let [source]=sources.as_slice()else{return Err(ModelError::Conflict("retained product release original producer cardinality"));};
        let physical=if source.get("state")==Some(&Value::String("frozen".into())){
            let physical=crate::completed_state::hold_physical_state_for_attempt(client.clone(),source_attempt,budget).await?;
            let selected=physical.inventory().references().iter().any(|reference|
                physical.inventory().installed(reference.logical_view).is_ok_and(|installed|
                    installed.mappings.iter().any(|mapping|*contribution==RecordId::new("compiler_contribution",mapping.physical.hex()))));
            if !selected{return Err(ModelError::Conflict("frozen product release contributor outside exact physical completion"));}
            Some(physical)
        }else{None};
        let identity=fresh_identity("retained-product-release")?;
        let job=RecordId::new("native_retirement",identity.hex());
        let mut bindings=Variables::new();
        bindings.insert("product",product.clone());bindings.insert("product_guard",guard_id(&Value::RecordId(product.clone()))?);
        bindings.insert("expected_product",expected.row().clone());bindings.insert("incarnation",expected.incarnation());
        bindings.insert("source_attempt",RecordId::new("native_attempt",source_attempt.hex()));bindings.insert("contribution",contribution.clone());
        bindings.insert("expected_source",source.clone());
        if let Some(physical)=&physical {
            bindings.insert("release_completion",physical.completion().record()?);
            bindings.insert("release_completion_bytes",physical.completion().value(true)?);
            bindings.insert("release_completion_guard",guard_id(&Value::RecordId(physical.completion().record()?))?);
            bindings.insert("release_completion_incarnation",physical.incarnation());
            bindings.insert("release_completion_identity",physical.completion().identity()?.hex());
        }else{
            for name in ["release_completion","release_completion_bytes","release_completion_guard","release_completion_incarnation","release_completion_identity"]{bindings.insert(name,Value::None);}
        }
        bindings.insert("job",job.clone());bindings.insert("item",retirement::item_id(&job,&product,expected.incarnation())?);
        bindings.insert("root_digest",ContentHash::of(&serde_json::to_vec(&("native-retirement/v4",vec![product])).map_err(ModelError::codec)?).hex());
        let _charge=budget.reserve("retained product release payload",crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(65536))?;
        effect(&client,None,RELEASE,bindings).await?;
        Ok(Some(identity))
    }.await;
    let mut completion=Completion::default();completion.step("retained product release reader close",reader.close().await);
    complete(result,completion)
}
fn record_hash(record:&RecordId)->Result<ContentHash,ModelError>{
    let surrealdb::types::RecordIdKey::String(key)=&record.key else{return Err(ModelError::Schema("retained product nomination identity"))};
    Ok(ContentHash(hex::decode(key).map_err(ModelError::codec)?.try_into().map_err(|_|ModelError::Schema("retained product nomination width"))?))
}
// Product/hold incidence and collector claim serialize on the same exact guard.
// The durable job/item exists in this transaction before warm retention ends.
const RELEASE:&str="fn::native_touch($product); LET $physical=SELECT * FROM ONLY $product_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $product FOR UPDATE; LET $source=SELECT * FROM ONLY $source_attempt FOR UPDATE; LET $owner=SELECT * FROM ONLY $contribution FOR UPDATE; IF $physical.phase!='active' OR $physical.incarnation!=$incarnation OR $actual!=$expected_product OR $actual.released!=false OR $actual.contribution!=$contribution OR $owner.attempt!=$source_attempt OR $source!=$expected_source OR $source.state NOT IN ['closed','abandoned','maintenance_fenced','frozen'] { THROW 'retained product release changed or producer active'; }; IF $source.state='frozen' { LET $completion_guard=SELECT * FROM ONLY $release_completion_guard FOR UPDATE; LET $completion=SELECT * FROM ONLY $release_completion FOR UPDATE; IF $release_completion=NONE OR $source.completed_identity!=$release_completion_identity OR $completion_guard.phase!='active' OR $completion_guard.incarnation!=$release_completion_incarnation OR $completion!=$release_completion_bytes { THROW 'retained frozen product physical completion changed'; }; }; fn::native_touch($job); fn::native_touch($item); IF (SELECT VALUE id FROM ONLY $job)!=NONE OR (SELECT VALUE id FROM ONLY $item)!=NONE { THROW 'retained product release nomination collision'; }; CREATE $job SET generation=$__generation,era=$__era,epoch=$__epoch,root_digest=$root_digest,lineage=$job,state='open',root_count=1,roots_registered=1,examined=0,retired=0,revision=0 RETURN NONE; CREATE $item SET job=$job,object=$product,incarnation=$incarnation,state='pending' RETURN NONE; fn::native_incidence($actual); UPDATE $product SET released=true,release_job=$job RETURN NONE;";

