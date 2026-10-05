use crate::domain::analysis::sources::{CapturedSources,SourceSnapshot};
/// Audit snapshots emitted only from actual declared R0 sources. Fields stay private; decoding
/// arbitrary raw store rows still cannot bypass the declaration-owned publication check.
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name=owner_table!("source_receipts"),validate=validate_source_receipt)]
pub struct SourceReceipt {
    #[model(key)] pub invocation:Id<AnalysisInvocation>,
    #[model(key)] relation:String,
    #[model(key)] producer:String,
    #[model(key)] model:ContentHash,
    #[model(key)] schedule:ContentHash,
    #[model(key)] content:ContentHash,
    #[model(key)] rows:i64,
    #[model(key)] physical:String,
    #[model(key)] prefix:Option<String>,
}
impl SourceReceipt {
    fn from_snapshot(invocation:Id<AnalysisInvocation>,source:&SourceSnapshot)->Self {Self {invocation,relation:source.relation.clone(),producer:source.producer.clone(),model:source.model,schedule:source.schedule,content:source.content,rows:source.rows,physical:source.physical.clone(),prefix:source.prefix.clone()}}
    fn snapshot(&self)->SourceSnapshot {SourceSnapshot {relation:self.relation.clone(),producer:self.producer.clone(),model:self.model,schedule:self.schedule,content:self.content,rows:self.rows,physical:self.physical.clone(),prefix:self.prefix.clone()}}
    pub fn source(&self)->SourceSnapshot {self.snapshot()}
}
fn validate_source_receipt(row:&SourceReceipt)->Result<(),ModelError> {if row.relation.is_empty() || row.producer.is_empty() || row.physical.is_empty() || row.rows<0 || row.prefix.as_deref().is_some_and(|name|stages::PublicationBoundary::from_name(name).is_none()) {return Err(invalid("source receipt has invalid metadata"));}Ok(())}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name=owner_table!("projection_inputs"),rule="analysis_projection_input",conclusion=invocation)]
pub struct ProjectionInput {#[model(key)] pub invocation:Id<AnalysisInvocation>,#[model(key,premise)] pub projection:Id<ProjectionDefinition>}
fn projection_digest(values:&std::collections::BTreeSet<Id<ProjectionDefinition>>)->ContentHash {let mut sink=KeySink::new("analysis-projection-inputs");for value in values {value.encode(&mut sink);}sink.finish()}
impl AnalysisInvocation {
    /// Bind semantic identity to admitted completed inputs, static projection meanings and exact
    /// nominal parent membership. CapturedSources never supplies new completion authority.
    #[allow(clippy::type_complexity,clippy::too_many_arguments,reason="Admission binds every semantic input of an invocation and returns the invocation with its owned input rows")]
    pub fn admitted(input:Id<InputRevision>,context:Id<AnalysisContext>,definition:Id<AnalysisDefinition>,subject:Option<Id<EntityRef>>,parents:impl IntoIterator<Item=Id<InvocationSource>>,sources:&CapturedSources,projections:impl IntoIterator<Item=Id<ProjectionDefinition>>,budget:&resources::ResourceBudget)->Result<(Self,Vec<AnalysisInput>,Vec<SourceReceipt>,Vec<ProjectionInput>),ModelError> {
        let mut charge=charged::StateCharge::new(budget,"analysis_invocation_admission");let mut parent_set=charged::ChargedSet::default();let mut projection_set=charged::ChargedSet::default();
        for parent in parents {if !parent_set.insert(&mut charge,parent)? {return Err(invalid("duplicate admitted invocation parent"));}}
        for projection in projections {if !projection_set.insert(&mut charge,projection)? {return Err(invalid("duplicate admitted invocation projection"));}}
        let row=Self {input,context,definition,subject,inputs:parent_digest(&parent_set),sources:sources.digest(),projections:projection_digest(&projection_set)};
        let source_bytes=sources.iter().try_fold(0usize,|bytes,source|bytes.checked_add(size_of::<SourceReceipt>())?.checked_add(source.heap_bytes())).ok_or_else(||invalid("source receipt allocation overflow"))?;
        let bytes=parent_set.len().checked_mul(size_of::<AnalysisInput>()).and_then(|n|n.checked_add(projection_set.len().checked_mul(size_of::<ProjectionInput>())?)).and_then(|n|n.checked_add(source_bytes)).ok_or_else(||invalid("invocation admission allocation overflow"))?;
        let _buffer=budget.reserve("analysis_invocation_admission",bytes)?;
        let parents=parent_set.iter().map(|parent|AnalysisInput {invocation:row.id(),parent:*parent}).collect();
        let receipts=sources.iter().map(|source|SourceReceipt::from_snapshot(row.id(),source)).collect();
        let projections=projection_set.iter().map(|projection|ProjectionInput {invocation:row.id(),projection:*projection}).collect();
        Ok((row,parents,receipts,projections))
    }
}
pub(crate) fn source_publication_checks()->Vec<PublicationInvariant> {let checks=vec![PublicationInvariant {revision: 1,name:owner_table!("invocation_sources"),inputs:vec![ValidationInput::of::<AnalysisInvocation>(&["id"]),ValidationInput::of::<SourceReceipt>(&["id"])],create:std::sync::Arc::new(|budget|Box::new(SourcePublicationCheck {charge:charged::StateCharge::new(budget,"analysis_source_publication"),invocations:Default::default(),sources:Default::default()}))}];checks}
struct SourcePublicationCheck {charge:charged::StateCharge,invocations:charged::ChargedMap<Id<AnalysisInvocation>,AnalysisInvocation>,sources:charged::ChargedMap<Id<AnalysisInvocation>,std::collections::BTreeMap<String,SourceSnapshot>>}
impl PublicationCheck for SourcePublicationCheck {
    fn visit(&mut self,relation:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError> {
        if relation==AnalysisInvocation::NAME {for row in AnalysisInvocation::decode(batch)? {if self.invocations.insert(&mut self.charge,row.id(),row)?.is_some() {return Err(ModelError::Conflict(AnalysisInvocation::NAME));}}return Ok(());}
        if relation==SourceReceipt::NAME {for row in SourceReceipt::decode(batch)? {let source=row.snapshot();let mut duplicate=false;self.sources.update(&mut self.charge,row.invocation,|sources|{duplicate=sources.insert(source.relation.clone(),source).is_some();})?;if duplicate {return Err(invalid("duplicate persisted source receipt"));}}return Ok(());}
        Err(invalid("undeclared source publication input"))
    }
    fn finish(self:Box<Self>,actual:&[stages::CompletedRelation],_profile:stages::Profile)->Result<(),ModelError> {
        let empty=std::collections::BTreeMap::new();let budget=self.charge.budget().ok_or_else(||invalid("source publication budget absent"))?;
        for (id,invocation) in self.invocations.iter() {let sources=self.sources.get(id).unwrap_or(&empty);if crate::domain::analysis::sources::digest(sources)!=invocation.sources {return Err(invalid("source receipt membership differs from invocation identity"));}crate::domain::analysis::sources::verify(sources,actual,budget)?;}
        for id in self.sources.keys() {if !self.invocations.contains_key(id) {return Err(invalid("source receipt has no owned invocation"));}}
        Ok(())
    }
}

pub(crate) fn source_publication_checks_refs() -> Vec<&'static str> { vec![owner_table!("invocation_sources"), owner_table!("coverage_frontier")] }
