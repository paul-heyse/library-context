//! Exact terminal product recovery. This earns protected physical reads only;
//! compiler storage separately executes current producer and semantic admission.
use super::*;
use surrealdb::types::RecordIdKey;
use crate::installed_view::{self,ContributorRealization,InstalledViewCompletion,HeldInstalledView};
use crate::completed_state::earning::{ViewSelection,root_members,verify_members,revision};
use lctx_model::domain::{charged::StateCharge,resources::ResourceBudget,
    completed::{CompletedContribution,CompletedView},recovery_closure::{RecoveryClosure,RecoveryTraversal},
    completion::{Completion,complete}};
use std::collections::{BTreeMap,BTreeSet};

/// Private fields retain the actual product and exact native namespace owners.
/// Neither this type nor a deserialized completion establishes semantic eligibility.
pub struct HeldTerminalProduct {
    client:Arc<Surreal<Client>>,pin:Arc<ReaderPin>,product:ExpectedRecord,
    contribution:ExpectedRecord,descriptor:CompletedContribution,
    generation:ContentHash,validity_epoch:i64,
    views:BTreeMap<ContentHash,Arc<HeldInstalledView>>,
    contributors:BTreeMap<ContentHash,CompletedContribution>,
    _charge:StateCharge,
}
// Factory and method cancellation must preserve the pin before a public cap or
// caller-owned drainage guard exists. A terminal logical failure may release;
// cancellation, panic and an unconfirmed result cannot.
struct RecoveryPinTerminal {pin:Arc<ReaderPin>,confirmed:bool}
impl RecoveryPinTerminal {
    fn new(pin:Arc<ReaderPin>)->Self{Self{pin,confirmed:false}}
    fn observe<T>(&mut self,result:&Result<T,ModelError>){self.confirmed=result.as_ref().err().is_none_or(|error|error.permits_storage_cleanup());}
}
impl Drop for RecoveryPinTerminal {fn drop(&mut self){if !self.confirmed{self.pin.retain_unknown();}}}

impl HeldTerminalProduct {
    pub fn descriptor(&self)->&CompletedContribution{&self.descriptor}
    pub fn contribution(&self)->&ExpectedRecord{&self.contribution}
    pub fn views(&self)->&BTreeMap<ContentHash,Arc<HeldInstalledView>>{&self.views}
    pub fn contributors(&self)->&BTreeMap<ContentHash,CompletedContribution>{&self.contributors}
    pub fn generation(&self)->ContentHash{self.generation}
    pub fn retain_unknown(&self){self.pin.retain_unknown();}
    pub fn reader(&self,budget:&ResourceBudget)->crate::NativeReader<()>{
        crate::NativeReader::private_pinned(self.client.clone(),self.pin.clone()).with_budget(budget)
    }
    /// Compare current captured source metadata with actual backing retained by
    /// this product's explicitly selected physical contributors. A missing row
    /// is not a completed-empty assertion; callers may use only returned matches
    /// as temporary reference targets and must reject missing referenced IDs.
    pub async fn verify_captured_row<R:lctx_model::domain::Record>(&self,row:&R,budget:&ResourceBudget)->Result<bool,ModelError>{
        use crate::adapter::GraphRow;
        if !matches!(R::NAME,"packages"|"releases"|"input_revisions"|"input_origins"|"input_acquisitions"|"corpus_libraries"|"input_distributions"|"distribution_verifications"|"environment_fingerprints"|"source_artifacts"|"artifact_ownership"|"unowned_artifacts"|"derived_artifacts"|"artifact_uses") {
            return Err(ModelError::Conflict("terminal capture companion family"));
        }
        row.validate()?;
        let mut charge=StateCharge::new(budget,"terminal captured canonical comparison");
        charge.grow(std::mem::size_of::<R>().saturating_add(lctx_model::domain::HeapSize::heap_bytes(row)).saturating_mul(32).saturating_add(65536))?;
        let adapter=crate::adapter::select(R::NAME)?;
        let batch=R::encode(std::slice::from_ref(row))?;
        let view=crate::codec::views_with_adapter(std::slice::from_ref(row),adapter)?.pop().ok_or(ModelError::Schema("terminal captured body"))?;
        let graph=adapter.graph(&batch)?.pop().ok_or(ModelError::Schema("terminal captured graph decode"))?;
        let (id,content,canonical,kind)=match graph {
            Some(GraphRow::Entity(entity))=>(crate::loader::entity_payload_id(&entity)?,entity.content(),serde_json::to_vec(&entity).map_err(ModelError::codec)?,Some((entity.kind() as i64,entity.subtype()))),
            Some(GraphRow::Assertion(assertion))=>(crate::loader::assertion_payload_id(&assertion)?,assertion.content(),serde_json::to_vec(&assertion).map_err(ModelError::codec)?,Some((assertion.kind as i64,None))),
            None=>{let canonical=serde_json::to_vec(&view.body).map_err(ModelError::codec)?;let content=ContentHash::of(&canonical);(crate::loader::payload_id("compiler_record",R::NAME,row.id().bytes(),content)?,content,canonical,None)},
        };
        let expected=crate::adapter::physical_row(id.clone(),content,canonical,kind,view)?;
        charge.grow(crate::loader::native_bytes(&expected).saturating_mul(4))?;
        let mut owners=BTreeSet::new();
        for view in self.views.values(){for mapping in &view.completion().mappings {
            charge.grow(512)?;owners.insert(RecordId::new("compiler_contribution",mapping.physical.hex()));
        }}
        let mut pin_terminal=RecoveryPinTerminal::new(self.pin.clone());
        let reader=self.reader(budget);
        let result=async {
            self.pin.ensure_live()?;
            let mut after=Value::None;let mut found=false;
            loop {
                let mut variables=Variables::new();variables.insert("capture_owners",owners.iter().cloned().collect::<Vec<_>>());
                variables.insert("capture_relation",R::NAME);variables.insert("capture_key",hex::encode(row.id().bytes()));variables.insert("capture_after",after.clone());
                let rows:Vec<Object>=reader.query_native("SELECT id,object AS address,object.* AS stored FROM native_hold WITH INDEX owner_holds WHERE owner IN $capture_owners AND object.semantic_type=$capture_relation AND object.semantic_key=$capture_key AND ($capture_after=NONE OR id>$capture_after) ORDER BY id LIMIT 1",variables).await?;
                let [actual]=rows.as_slice()else{if !rows.is_empty(){return Err(ModelError::Conflict("terminal captured bounded backing page"));}break;};
                after=actual.get("id").cloned().ok_or(ModelError::Schema("terminal captured backing cursor"))?;
                if actual.get("address")!=Some(&Value::RecordId(id.clone())) || actual.get("stored")!=Some(&expected){return Err(ModelError::Conflict("terminal captured native payload differs"));}
                self.pin.protect(id.clone()).await?;
                let protected=held::inspect_expected_read(&reader,id.clone(),budget).await?.ok_or(ModelError::Conflict("terminal captured native backing disappeared"))?;
                if protected.row()!=&expected{return Err(ModelError::Conflict("terminal captured protected bytes changed"));}
                found=true;
            }
            Ok(found)
        }.await;
        let mut terminal=Completion::default();terminal.step("terminal captured backing reader close",reader.close().await);
        let result=complete(result,terminal);
        pin_terminal.observe(&result);
        result
    }
    /// Repeat actual guard/bytes and installation checks; an old original counter
    /// or a matching logical contributor in another namespace cannot satisfy them.
    pub async fn ensure_current(&self,budget:&ResourceBudget)->Result<(),ModelError>{
        let mut pin_terminal=RecoveryPinTerminal::new(self.pin.clone());
        let reader=self.reader(budget);
        let result=async {
            self.pin.ensure_live()?;
            for original in [&self.product,&self.contribution] {
                let actual=held::inspect_expected_read(&reader,original.record().clone(),budget).await?
                    .ok_or(ModelError::Conflict("terminal product held record disappeared"))?;
                if actual.incarnation()!=original.incarnation() || actual.row()!=original.row(){
                    return Err(ModelError::Conflict("terminal product held bytes/incarnation changed"));
                }
            }
            check_installation(&reader,self.generation,self.validity_epoch).await
        }.await;
        let mut terminal=Completion::default();terminal.step("terminal product current reader close",reader.close().await);
        let result=complete(result,terminal);pin_terminal.observe(&result);result
    }
}
fn terminal_source(source:&Object,generation:ContentHash)->Result<(),ModelError>{
    if authorization::hash_field(source,"generation")?!=generation
        || authorization::int_field(source,"epoch")?<=0
        || !matches!(source.get("state"),Some(Value::String(state)) if matches!(state.as_str(),"closed"|"abandoned"|"maintenance_fenced"|"frozen")) {
        return Err(ModelError::Conflict("retained product producer is not terminal/current"));
    }Ok(())
}
async fn installation(reader:&crate::NativeReader<()>)->Result<(ContentHash,i64),ModelError>{
    let rows:Vec<Object>=reader.query_native("SELECT generation,validity_epoch FROM native_installation:current",Variables::new()).await?;
    let [row]=rows.as_slice()else{return Err(ModelError::Conflict("terminal product installation cardinality"));};
    let epoch=authorization::int_field(row,"validity_epoch")?;
    if epoch<=0{return Err(ModelError::Conflict("terminal product installation epoch"));}
    Ok((authorization::hash_field(row,"generation")?,epoch))
}
async fn check_installation(reader:&crate::NativeReader<()>,generation:ContentHash,epoch:i64)->Result<(),ModelError>{
    if installation(reader).await?!=(generation,epoch){return Err(ModelError::Conflict("terminal product installation changed"));}Ok(())
}

/// The supplied address is discovery data. This constructor independently protects
/// and reads the actual product, singleton output witnesses and full selected closure.
pub async fn hold_terminal_product(client:Arc<Surreal<Client>>,product:RecordId,request:ContentHash,
    budget:&ResourceBudget)->Result<HeldTerminalProduct,ModelError>{
    if product.table.as_str()!="native_product"{return Err(ModelError::Conflict("terminal product family"));}
    let pin=ReaderPin::acquire(client.clone(),&[]).await?;
    let mut pin_terminal=RecoveryPinTerminal::new(pin.clone());
    let reader=crate::NativeReader::private_pinned(client.clone(),pin.clone()).with_budget(budget);
    let result=async {
        pin.protect(product.clone()).await?;
        let product=held::inspect_expected_read(&reader,product,budget).await?
            .ok_or(ModelError::Conflict("terminal product disappeared before protection"))?;
        let row=product.row().as_object().ok_or(ModelError::Schema("terminal product row"))?;
        if row.get("request")!=Some(&Value::String(request.hex())) || row.get("released")!=Some(&Value::Bool(false)) {
            return Err(ModelError::Conflict("terminal product request/retention changed"));
        }
        let Some(Value::RecordId(address))=row.get("contribution")else{return Err(ModelError::Schema("terminal product contributor"));};
        if address.table.as_str()!="compiler_contribution"{return Err(ModelError::Schema("terminal product contributor family"));}
        pin.protect(address.clone()).await?;
        let contribution=held::inspect_expected_read(&reader,address.clone(),budget).await?
            .ok_or(ModelError::Conflict("terminal product contributor missing"))?;
        crate::completed_rows::validate_state_row("compiler_contribution",contribution.row())?;
        let mut charge=StateCharge::new(budget,"terminal product exact descriptors");
        charge.grow(crate::loader::native_bytes(contribution.row()).saturating_mul(32).saturating_add(65536))?;
        let descriptor:CompletedContribution=installed_view::audit::descriptor(contribution.row())?;
        let logical=descriptor.identity()?;
        let stored=contribution.row().as_object().ok_or(ModelError::Schema("terminal contribution row"))?;
        if stored.get("completed")!=Some(&Value::Bool(true)) || stored.get("logical")!=Some(&Value::String(logical.hex())) {
            return Err(ModelError::Conflict("terminal product contributor not complete"));
        }
        let Some(Value::RecordId(source))=stored.get("attempt")else{return Err(ModelError::Schema("terminal contributor original attempt"));};
        let (generation,validity_epoch)=installation(&reader).await?;
        let mut vars=Variables::new();vars.insert("source",source.clone());
        let sources:Vec<Object>=reader.query_native("SELECT * FROM $source",vars).await?;
        let [source_row]=sources.as_slice()else{return Err(ModelError::Conflict("terminal product original owner cardinality"));};
        terminal_source(source_row,generation)?;
        let RecordIdKey::String(source_key)=&source.key else{return Err(ModelError::Schema("terminal product source identity"));};
        if product.record()!=&RecordId::new("native_product",format!("{}_{}",request.hex(),source_key)) {
            return Err(ModelError::Conflict("terminal product original owner address"));
        }
        crate::completed_state::require_hold(&reader,product.record(),contribution.record()).await?;
        let mut pending=BTreeMap::new();let mut dependencies=BTreeMap::new();let mut outputs=BTreeSet::new();
        let mut variables=Variables::new();variables.insert("product",product.record().clone());
        let mut stream=reader.stream_prepared(crate::prepared::PreparedQuery::new(variables,vec![],vec![
            "SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$product AND record::table(object)='compiler_view_completion' ORDER BY id".into()])?)?;
        let output_result=async {
            while let Some(value)=stream.next().await? {
                let Value::RecordId(address)=value else{return Err(ModelError::Schema("terminal output completion address"));};
                pin.protect(address.clone()).await?;
                let expected=held::inspect_expected_read(&reader,address,budget).await?.ok_or(ModelError::Conflict("terminal output completion missing"))?;
                charge.grow(crate::loader::native_bytes(expected.row()).saturating_mul(32).saturating_add(65536))?;
                let installed=InstalledViewCompletion::from_row(expected.row())?;
                if installed.generation!=generation || installed.validity_epoch!=validity_epoch {
                    return Err(ModelError::Conflict("terminal witness installation"));
                }
                let singleton=descriptor.outputs.get(&installed.view.relation).map(|output|
                    CompletedView::new(installed.view.relation.clone(),BTreeSet::from([logical]),output.rows)).transpose()?;
                if singleton.as_ref()!=Some(&installed.view) {
                    if dependencies.insert(installed.view.identity,(installed,expected)).is_some(){return Err(ModelError::Conflict("duplicate terminal dependency witness"));}
                    continue;
                }
                if !installed.mappings.iter().any(|m|m.view==installed.view.identity && m.logical==logical
                        && RecordId::new("compiler_contribution",m.physical.hex())==*contribution.record() && m.incarnation==contribution.incarnation())
                    || !outputs.insert(installed.view.relation.clone()) {
                    return Err(ModelError::Conflict("terminal exact singleton output witness"));
                }
                pending.insert(installed.view.identity,(installed,expected));
            }Ok::<_,ModelError>(())
        }.await;
        let mut drained=Completion::default();drained.step("terminal output witness stream drain",stream.drain_transport().await);
        complete(output_result,drained)?;
        if outputs!=descriptor.outputs.keys().cloned().collect(){return Err(ModelError::Conflict("terminal complete output witness inventory"));}
        let mut views=BTreeMap::new();let mut contributors=BTreeMap::new();
        for (installed,expected) in pending.into_values() {
            // Protect exact named physical copies and logical views before auditing.
            // A subsequently recreated copy fails the independently re-read mapping.
            for mapping in &installed.mappings {
                pin.protect(RecordId::new("compiler_contribution",mapping.physical.hex())).await?;
                pin.protect(RecordId::new("compiler_view",mapping.view.hex())).await?;
            }
            let closure=installed_view::audit::inspect_closure(&reader,&installed.view,&installed.mappings,budget).await?;
            installed_view::audit::audit_memberships(&reader,&closure,budget).await?;
            let aliases=installed_view::audit::audit_aliases(&reader,&closure,budget).await?;
            if installed_view::audit::logical_roots(&reader,&closure,&[],&aliases,budget).await?!=installed.closure {
                return Err(ModelError::Conflict("terminal singleton actual physical closure"));
            }
            verify_namespace(&reader,&installed,budget).await?;
            for (id,owner) in closure.logical.contributions(){
                charge.grow(serde_json::to_vec(owner).map_err(ModelError::codec)?.len().saturating_mul(4).saturating_add(1024))?;
                if contributors.insert(*id,owner.clone()).is_some_and(|old|old!=*owner){return Err(ModelError::Conflict("terminal dependency descriptor collision"));}
            }
            let root_id=installed.view.identity;
            for view in closure.logical.views().values() {
                if views.contains_key(&view.identity){continue;}
                if view.identity==root_id {continue;}
                let selected=subclosure(&closure.logical,view)?;
                let mappings=installed.mappings.iter().filter(|m|selected.views().contains_key(&m.view)).cloned().collect::<Vec<_>>();
                let held=earn_dependency(&client,&reader,&pin,&product,generation,validity_epoch,view,&mappings,dependencies.remove(&view.identity),budget).await?;
                charge.grow(4096)?;views.insert(view.identity,Arc::new(held));
            }
            let held=installed_view::recovery_view(client.clone(),pin.clone(),installed,expected,budget)?;
            charge.grow(4096)?;views.insert(root_id,Arc::new(held));
        }
        if !dependencies.is_empty(){return Err(ModelError::Conflict("terminal extraneous product dependency witness"));}
        let result=HeldTerminalProduct{client:client.clone(),pin:pin.clone(),product,contribution,descriptor,
            generation,validity_epoch,views,contributors,_charge:charge};
        result.ensure_current(budget).await?;Ok(result)
    }.await;
    let mut terminal=Completion::default();terminal.step("terminal product acquisition reader close",reader.close().await);
    let result=complete(result,terminal);
    pin_terminal.observe(&result);
    result
}
fn subclosure(all:&RecoveryClosure,root:&CompletedView)->Result<RecoveryClosure,ModelError>{
    let mut traversal=RecoveryTraversal::for_views(vec![root.clone()])?;
    while let Some(view)=traversal.next_view()? {
        let owners=view.contributions.iter().map(|id|all.contributions().get(id).cloned()
            .ok_or(ModelError::Conflict("terminal subclosure contributor"))).collect::<Result<Vec<_>,_>>()?;
        traversal.include_view(&view,view.clone(),owners,|id|all.views().get(&id).cloned()
            .ok_or(ModelError::Conflict("terminal subclosure view")))?;
    }traversal.finish()
}
async fn verify_namespace(reader:&crate::NativeReader<()>,installed:&InstalledViewCompletion,budget:&ResourceBudget)->Result<(),ModelError>{
    let root=RecordId::new("compiler_view_realization",installed.realization.hex());
    let expected=held::inspect_expected_read(reader,root.clone(),budget).await?.ok_or(ModelError::Conflict("terminal sealed namespace missing"))?;
    if expected.incarnation()!=installed.realization_incarnation || expected.row()!=&installed.realization_value()? {
        return Err(ModelError::Conflict("terminal sealed namespace bytes/incarnation"));
    }
    let selection=ViewSelection{view:installed.view.clone(),mappings:installed.mappings.clone()};
    let before=revision(reader,&root).await?;
    let members=root_members(reader,&selection,installed.realization,budget).await?;
    if verify_members(reader,installed.realization,&members,installed.view.rows,budget).await?!=installed.membership_content
        || revision(reader,&root).await?!=before {
        return Err(ModelError::Conflict("terminal actual namespace EOF changed"));
    }Ok(())
}

// Recovery witness construction is fixed to exact selected immutable content.
// It cannot write canonical payload, an artifact receipt or receiving-attempt state.
const FENCE:&str="LET $pg=SELECT * FROM ONLY $product_guard FOR UPDATE; LET $product=SELECT * FROM ONLY $product_record FOR UPDATE; IF $pg.phase!='active' OR $pg.incarnation!=$product_incarnation OR $product!=$product_bytes OR $product.released!=false { THROW 'terminal recovery product changed'; }; IF $__installation.generation!=$recovery_generation OR $__installation.validity_epoch!=$recovery_epoch { THROW 'terminal recovery installation changed'; };";
fn bind_product(product:&ExpectedRecord,variables:&mut Variables)->Result<(),ModelError>{
    variables.insert("product_record",product.record().clone());variables.insert("product_guard",guard_id(&Value::RecordId(product.record().clone()))?);
    variables.insert("product_incarnation",product.incarnation());variables.insert("product_bytes",product.row().clone());Ok(())
}
async fn install(client:&Surreal<Client>,pin:&ReaderPin,product:&ExpectedRecord,generation:ContentHash,epoch:i64,row:Option<Value>,
    edges:Vec<(RecordId,RecordId)>,budget:&ResourceBudget)->Result<(),ModelError>{
    if row.as_ref().is_some_and(|row|!matches!(row.as_object().and_then(|r|r.get("id")),Some(Value::RecordId(id)) if matches!(id.table.as_str(),"compiler_view_realization"|"compiler_view_member"|"compiler_view_completion"))) {
        return Err(ModelError::Conflict("terminal recovery witness family"));
    }
    let _charge=budget.reserve("terminal recovery witness effect",row.as_ref().map_or(0,crate::loader::native_bytes).saturating_add(edges.len().saturating_mul(1024)).saturating_mul(32).saturating_add(131072))?;
    let mut sql=String::from(FENCE);let mut variables=Variables::new();bind_product(product,&mut variables)?;variables.insert("recovery_generation",generation.hex());variables.insert("recovery_epoch",epoch);
    if let Some(row)=row {let (plan,bindings)=immutable_row_plan(None,None,vec![row])?.ok_or(ModelError::Schema("terminal witness row plan"))?;sql.push_str(&plan);merge_plan_bindings(&mut variables,bindings)?;}
    if let Some((_,plan,bindings))=hold_plan(None,edges)?{sql.push_str(&plan);merge_plan_bindings(&mut variables,bindings)?;}
    effect_for_owner(client,EffectOwner::Pin(RecordId::new("native_pin",pin.identity.hex())),&sql,variables).await
}
async fn earn_dependency(client:&Arc<Surreal<Client>>,reader:&crate::NativeReader<()>,pin:&Arc<ReaderPin>,product:&ExpectedRecord,
    generation:ContentHash,epoch:i64,view:&CompletedView,mappings:&[ContributorRealization],original:Option<(InstalledViewCompletion,ExpectedRecord)>,budget:&ResourceBudget)->Result<HeldInstalledView,ModelError>{
    let closure=installed_view::audit::inspect_closure(reader,view,mappings,budget).await?;
    installed_view::audit::audit_memberships(reader,&closure,budget).await?;
    let aliases=installed_view::audit::audit_aliases(reader,&closure,budget).await?;
    let roots=installed_view::audit::logical_roots(reader,&closure,&[],&aliases,budget).await?;
    let realization=installed_view::realization_id(view,mappings)?;let root=RecordId::new("compiler_view_realization",realization.hex());
    let selection=ViewSelection{view:view.clone(),mappings:mappings.to_vec()};
    let members=root_members(reader,&selection,realization,budget).await?;
    if let Some((installed,expected))=original {
        if installed.view!=*view || installed.mappings!=mappings || installed.closure!=roots
            || installed.realization!=realization || installed.generation!=generation || installed.validity_epoch!=epoch {
            return Err(ModelError::Conflict("terminal original dependency witness metadata"));
        }
        verify_namespace(reader,&installed,budget).await?;
        crate::completed_state::require_hold(reader,product.record(),expected.record()).await?;
        pin.protect(root.clone()).await?;
        return installed_view::recovery_view(client.clone(),pin.clone(),installed,expected,budget);
    }
    let previous=held::inspect_expected_read(reader,root.clone(),budget).await?;
    let declaration=if let Some(previous)=&previous {
        if previous.row()!=&installed_view::realization_declaration(view,mappings,true)? && previous.row()!=&installed_view::realization_declaration(view,mappings,false)?{return Err(ModelError::Conflict("terminal existing dependency namespace not sealed/exact"));}
        previous.row().clone()
    }else{installed_view::realization_declaration(view,mappings,false)?};
    install(client,pin,product,generation,epoch,Some(declaration),vec![(product.record().clone(),root.clone()),(RecordId::new("native_pin",pin.identity.hex()),root.clone()),(root.clone(),RecordId::new("compiler_view",view.identity.hex()))],budget).await?;
    for expected in closure.contributors.values(){
        // Pin retention and a repeated guard/bytes observation precede every edge.
        let actual=held::inspect_expected_read(reader,expected.record().clone(),budget).await?.ok_or(ModelError::Conflict("terminal dependency contributor disappeared"))?;
        if actual.incarnation()!=expected.incarnation() || actual.row()!=expected.row(){return Err(ModelError::Conflict("terminal dependency contributor changed"));}
        install(client,pin,product,generation,epoch,None,vec![(root.clone(),expected.record().clone())],budget).await?;
    }
    if previous.as_ref().is_none_or(|row|row.row().as_object().and_then(|r|r.get("sealed"))!=Some(&Value::Bool(true))){
        let mut cursor=members.cursor_with_budget(budget)?;
        while let Some(member)=cursor.next_row()? {
            let row=member.as_object().ok_or(ModelError::Schema("terminal lowered member"))?;
            let Some(Value::RecordId(id))=row.get("id")else{return Err(ModelError::Schema("terminal lowered member id"));};
            let Some(Value::RecordId(node))=row.get("node")else{return Err(ModelError::Schema("terminal lowered member node"));};
            install(client,pin,product,generation,epoch,Some(member.clone()),vec![(root.clone(),id.clone()),(id.clone(),node.clone())],budget).await?;
        }
    }
    let expected=held::inspect_expected_read(reader,root.clone(),budget).await?.ok_or(ModelError::Conflict("terminal dependency realization absent"))?;
    let before=revision(reader,&root).await?;
    let membership_content=verify_members(reader,realization,&members,view.rows,budget).await?;
    let mut variables=Variables::new();bind_product(product,&mut variables)?;variables.insert("recovery_generation",generation.hex());variables.insert("recovery_epoch",epoch);
    variables.insert("realization_hash",realization.hex());variables.insert("realization",root.clone());variables.insert("realization_guard",guard_id(&Value::RecordId(root.clone()))?);
    variables.insert("incarnation",expected.incarnation());variables.insert("bytes",expected.row().clone());variables.insert("revision",before);
    let sql=format!("{FENCE} LET $existing=SELECT VALUE object FROM native_hold WITH INDEX owner_holds WHERE owner=$product_record AND record::table(object)='compiler_view_completion' AND object IN (SELECT VALUE owner FROM native_hold WITH INDEX object_holds WHERE object=$realization) LIMIT 1; IF array::len($existing)!=0 {{ THROW 'terminal dependency witness concurrently retained'; }}; fn::native_touch($realization); LET $predicate=SELECT VALUE revision FROM ONLY type::record('native_predicate',[$realization]); LET $guard=SELECT * FROM ONLY $realization_guard FOR UPDATE; LET $actual=SELECT * FROM ONLY $realization FOR UPDATE; IF $predicate!=$revision+1 OR $guard.phase!='active' OR $guard.incarnation!=$incarnation OR $actual!=$bytes {{ THROW 'terminal dependency EOF audit changed'; }}; IF !$actual.sealed {{ UPDATE $realization SET sealed=true RETURN NONE; }}");
    let installed=InstalledViewCompletion{version:1,generation,validity_epoch:epoch,realization_incarnation:expected.incarnation(),
        view:view.clone(),realization,member_codec:installed_view::MEMBER_CODEC,mappings:mappings.to_vec(),closure:roots,
        membership_rows:view.rows,membership_content,frozen_revision:before.checked_add(1).ok_or(ModelError::Schema("terminal dependency revision overflow"))?};
    let (row_plan,row_bindings)=immutable_row_plan(None,None,vec![installed.value()?])?.ok_or(ModelError::Schema("terminal dependency completion plan"))?;
    merge_plan_bindings(&mut variables,row_bindings)?;
    let (_,holds,hold_bindings)=hold_plan(None,vec![(product.record().clone(),installed.record()?),(RecordId::new("native_pin",pin.identity.hex()),installed.record()?),(installed.record()?,root)])?.ok_or(ModelError::Schema("terminal dependency completion ownership"))?;
    merge_plan_bindings(&mut variables,hold_bindings)?;
    let sql=format!("{sql} {row_plan} {holds}");
    let _effect=budget.reserve("terminal dependency atomic completion",crate::loader::native_bytes(&installed.value()?).saturating_mul(32).saturating_add(sql.len()).saturating_add(131072))?;
    effect_for_owner(client,EffectOwner::Pin(RecordId::new("native_pin",pin.identity.hex())),&sql,variables).await?;
    let expected=held::inspect_expected_read(reader,installed.record()?,budget).await?.ok_or(ModelError::Conflict("terminal reconstructed dependency witness absent"))?;
    if expected.row()!=&installed.value()?{return Err(ModelError::Conflict("terminal reconstructed dependency witness bytes"));}
    installed_view::recovery_view(client.clone(),pin.clone(),installed,expected,budget)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn recovery_factory_guard_retains_unknown_before_cap_publication(){
        let pin=ReaderPin::test_detached(false);
        drop(RecoveryPinTerminal::new(pin.clone()));
        assert!(pin.test_uncertain());
        let confirmed=ReaderPin::test_detached(false);
        let mut guard=RecoveryPinTerminal::new(confirmed.clone());guard.observe(&Ok::<_,ModelError>(()));drop(guard);
        assert!(!confirmed.test_uncertain());confirmed.retain_unknown();
    }
    #[test]
    fn terminal_product_requires_actual_terminal_current_source(){
        let generation=ContentHash::of(b"installed current generation");
        let mut row=Object::new();row.insert("generation",generation.hex());row.insert("epoch",1_i64);
        for state in ["closed","abandoned","maintenance_fenced","frozen"]{
            row.insert("state",state);terminal_source(&row,generation).unwrap();
        }
        for state in ["open","closing","preparing","failed",""]{
            row.insert("state",state);assert!(terminal_source(&row,generation).is_err());
        }
        row.insert("state","closed");row.insert("epoch",0_i64);assert!(terminal_source(&row,generation).is_err());
        row.insert("epoch",1_i64);assert!(terminal_source(&row,ContentHash::of(b"foreign generation")).is_err());
    }
}
