//! Shared proof of a source parameter binding's entry value at one exact read.
//! This proves binding identity, never mutable object-state stability.
use crate::domain::{*, assertion::*, attribution::*, declarations::*, flow::*, lexical::*, normalized::{Rows,entities::*}, source::*, syntax::{SyntaxPlacement,SyntaxPlacementSupport}, value::*};

#[macro_export]
macro_rules! entry_value_inputs {($apply:ident)=>{$apply! {
    artifacts: $crate::domain::source::SourceArtifact,
    modules: $crate::domain::source::Module,
    scopes: $crate::domain::source::CoverageScope,
    occurrences: $crate::domain::source::Occurrence,
    qualifications: $crate::domain::assertion::AssertionQualification,
    predicates: $crate::domain::value::Predicate,
    atoms: $crate::domain::conditions::EvaluationAtom,
    conditions: $crate::domain::conditions::Condition,
    condition_nodes: $crate::domain::conditions::ConditionNode,
    roots: $crate::domain::value::PlaceRoot,
    paths: $crate::domain::value::AccessPath,
    places: $crate::domain::value::Place,
    formals: $crate::domain::normalized::entities::ParameterEntity,
    links: $crate::domain::normalized::entities::ParameterEntityLink,
    callables: $crate::domain::normalized::entities::CallableEntity,
    refs: $crate::domain::normalized::entities::EntityRef,
    owners: $crate::domain::normalized::entities::OccurrenceOwnership,
    declarations: $crate::domain::declarations::ParameterDeclaration,
    declaration_supports: $crate::domain::declarations::ParameterDeclarationSupport,
    symbol_declarations: $crate::domain::declarations::SymbolDeclaration,
    symbol_declaration_supports: $crate::domain::declarations::SymbolDeclarationSupport,
    signatures: $crate::domain::calls::Signature,
    parameters: $crate::domain::calls::SignatureParameter,
    symbols: $crate::domain::calls::ProviderSymbol,
    lexical_scopes: $crate::domain::lexical::LexicalScope,
    placements: $crate::domain::syntax::SyntaxPlacement,
    placement_supports: $crate::domain::syntax::SyntaxPlacementSupport,
    uses: $crate::domain::flow::FlowUse,
    use_observations: $crate::domain::flow::FlowUseObservation,
    use_supports: $crate::domain::flow::FlowUseSupport,
    definitions: $crate::domain::flow::FlowDefinition,
    definition_observations: $crate::domain::flow::FlowDefinitionObservation,
    definition_supports: $crate::domain::flow::FlowDefinitionSupport,
    targets: $crate::domain::flow::ReachingDefinition,
    reaching: $crate::domain::flow::FlowReachingObservation,
    reaching_supports: $crate::domain::flow::FlowReachingSupport,
    providers: $crate::domain::attribution::Provider,
    runs: $crate::domain::attribution::ProviderRun,
    surfaces: $crate::domain::assertion::ProviderSurface,
    evidence: $crate::domain::assertion::Evidence,
    coverage: $crate::domain::attribution::ProviderCoverage,
    regions: $crate::domain::flow::FlowRegionObservation,
    region_supports: $crate::domain::flow::FlowRegionSupport,
    values: $crate::domain::flow::FlowValueObservation,
    value_supports: $crate::domain::flow::FlowValueSupport,
    leaves: $crate::domain::flow::FlowTestLeafObservation,
    leaf_supports: $crate::domain::flow::FlowTestLeafSupport,
}};}
macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{
    pub struct EntryData {$(pub $field:Rows<$ty>,)*}
    impl EntryData {
        pub fn new(budget:&resources::ResourceBudget)->Self {Self{$($field:Rows::new(budget),)*}}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})* Ok(false)}
        pub fn validation_inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
    }
};}
crate::entry_value_inputs!(inputs);

#[derive(Debug,Clone,Copy)]
pub struct EntryRequest {pub owner:Id<EntityRef>,pub formal:Id<ParameterEntity>,pub access:Id<Occurrence>,pub context:Id<AnalysisContext>,pub run:Id<ProviderRun>}
#[derive(Debug,Clone,PartialEq,Eq,Domain)]
#[model(name="entry_value_witnesses", rule="parameter_entry_value", conclusion=formal, invariants=entry_invariants)]
pub struct EntryValueWitness {
    #[model(key)] pub owner:Id<EntityRef>,
    #[model(key)] pub formal:Id<ParameterEntity>,
    #[model(key)] pub access:Id<Occurrence>,
    #[model(key)] pub context:Id<AnalysisContext>,
    #[model(key,premise)] pub run:Id<ProviderRun>,
    #[model(key,premise)] pub access_source:Id<EntryAccessSource>,
    pub link:Id<ParameterEntityLink>,
    pub declaration_support:Id<ParameterDeclarationSupport>,
    pub owner_support:Id<SymbolDeclarationSupport>,
    pub use_observation:Id<FlowUseObservation>,
    pub use_support:Id<FlowUseSupport>,
    pub reaching:Id<FlowReachingObservation>,
    pub reaching_support:Id<FlowReachingSupport>,
    pub definition:Id<FlowDefinitionObservation>,
    pub definition_support:Id<FlowDefinitionSupport>,
    pub parameter_placement:Option<Id<SyntaxPlacement>>,
    pub parameter_placement_support:Option<Id<SyntaxPlacementSupport>>,
    pub coverage:Id<ProviderCoverage>,
}
/// Opaque result of replay. Stored witness rows alone cannot construct entry authority.
/// ```
/// use lctx_model::domain::conditions::entry::DerivedEntryValue;
/// fn inspect(proof:&DerivedEntryValue) {let _=proof.witness();}
/// ```
/// ```compile_fail
/// use lctx_model::domain::conditions::entry::DerivedEntryValue;
/// fn forge(mut proof:DerivedEntryValue) {proof.parameter=todo!();}
/// ```
pub struct DerivedEntryValue {source:EntryAccessSource,status:crate::domain::analysis::policy::EvidenceStatus,qualification:AssertionQualification,condition:super::Diagram,witness:EntryValueWitness,parameter:Id<crate::domain::calls::SignatureParameter>,root:PlaceRoot,place:Place,_charge:std::sync::Arc<charged::StateCharge>}
impl DerivedEntryValue {
    pub fn source(&self)->&EntryAccessSource {&self.source}
    pub fn evidence_status(&self)->crate::domain::analysis::policy::EvidenceStatus{self.status}
    pub fn qualification(&self)->&AssertionQualification{&self.qualification}
    pub fn condition(&self)->&super::Diagram{&self.condition}
    pub fn witness(&self)->&EntryValueWitness {&self.witness}
    pub fn parameter(&self)->Id<crate::domain::calls::SignatureParameter> {self.parameter}
    pub fn root(&self)->&PlaceRoot {&self.root}
    pub fn place(&self)->&Place {&self.place}
    pub(crate) fn allowance(&self)->std::sync::Arc<charged::StateCharge>{self._charge.clone()}
}
fn need<R:Record>(rows:&Rows<R>,id:Id<R>)->Result<&R,ObligationKind> {rows.get(id).ok_or(ObligationKind::MissingEvidence)}
fn inside(child:&Occurrence,parent:&Occurrence)->bool {child.source==parent.source && child.structural_path.len()>parent.structural_path.len() && child.structural_path.starts_with(&parent.structural_path) && child.start>=parent.start && child.end<=parent.end}
#[derive(Clone,Copy)]
struct AccessFrame {owner:Id<EntityRef>,access:Id<Occurrence>,context:Id<AnalysisContext>,run:Id<ProviderRun>}
impl From<EntryRequest> for AccessFrame {fn from(request:EntryRequest)->Self{Self{owner:request.owner,access:request.access,context:request.context,run:request.run}}}
fn exact(data:&EntryData,id:Id<AssertionQualification>,request:impl Into<AccessFrame>)->Result<&AssertionQualification,ObligationKind> {
let request=request.into();
    let q=need(&data.qualifications,id)?;
    if q.context!=request.context {return Err(ObligationKind::IncompatibleContexts);}
    if q.modality!=Modality::Definite || q.approximation!=Approximation::Exact {return Err(ObligationKind::Approximation);}
    let input=need(&data.runs,request.run)?.input;
    let source=need(&data.occurrences,request.access)?.source;
    let artifact=need(&data.artifacts,source)?;
    let covered=match need(&data.scopes,q.scope)? {CoverageScope::Input{input:owner}=>*owner==input && artifact.input==input,CoverageScope::Artifact{artifact:owner}=>*owner==source && artifact.input==input,CoverageScope::Module{module}=>need(&data.modules,*module)?.source==source && artifact.input==input,_=>false};
    if !covered {return Err(ObligationKind::MissingEvidence);}Ok(q)
}
fn supported<S:Support>(data:&EntryData,support:&S,request:impl Into<AccessFrame>)->Result<bool,ObligationKind> {
let request=request.into();
    let a=support.attribution().ok_or(ObligationKind::MissingEvidence)?;
    if a.run!=request.run {return Ok(false);}
    let run=need(&data.runs,a.run)?;let surface=need(&data.surfaces,a.surface)?;
    if run.context!=request.context || surface.provider!=run.provider || surface.family!=S::Assertion::FAMILY || !(a.fidelity==Fidelity::NativeStructural || S::Assertion::FAMILY==FactFamily::Signatures && a.fidelity==Fidelity::ReportProjection) || data.providers.get(run.provider).is_none(){return Err(ObligationKind::MissingEvidence);}
    match need(&data.evidence,a.evidence)? {Evidence::Occurrence{occurrence}=>{if *occurrence!=request.access && need(&data.occurrences,*occurrence)?.source!=need(&data.occurrences,request.access)?.source{return Err(ObligationKind::MissingEvidence);}},Evidence::SourceSpan{source,..}=>{if *source!=need(&data.occurrences,request.access)?.source{return Err(ObligationKind::MissingEvidence);}},Evidence::Invocation{run} if *run==a.run=>{},_=>return Err(ObligationKind::MissingEvidence)}
    Ok(true)
}
impl EntryValueWitness {
    pub fn request(&self)->EntryRequest {EntryRequest{owner:self.owner,formal:self.formal,access:self.access,context:self.context,run:self.run}}
    /// A complete selected provider/run reaching set must contain exactly the parameter definition.
    /// Unsupported or incomplete premises return an obligation, never a false theorem.
    pub fn derive(data:&EntryData,request:EntryRequest,budget:&resources::ResourceBudget)->Result<Result<DerivedEntryValue,ObligationKind>,ModelError> {
        Self::derive_source(data,request,None,budget)
    }
    pub fn derive_for(data:&EntryData,request:EntryRequest,source:&EntryAccessSource,budget:&resources::ResourceBudget)->Result<Result<DerivedEntryValue,ObligationKind>,ModelError>{Self::derive_source(data,request,Some(source),budget)}
    fn derive_source(data:&EntryData,request:EntryRequest,source:Option<&EntryAccessSource>,budget:&resources::ResourceBudget)->Result<Result<DerivedEntryValue,ObligationKind>,ModelError>{
        let mut charge=charged::StateCharge::new(budget,"entry-value-derivation");charge.grow(1024)?;
        let mut resource_error=None;let result=(||{
            let run=need(&data.runs,request.run)?;if run.context!=request.context {return Err(ObligationKind::IncompatibleContexts);}
            let ParameterEntity::Source{declaration}=need(&data.formals,request.formal)? else{return Err(ObligationKind::EntryValueUnknown)};
            let EntityRef::Callable{callable}=need(&data.refs,request.owner)? else{return Err(ObligationKind::EntryValueUnknown)};
            let CallableEntity::Source{declaration:owner,..}=need(&data.callables,*callable)? else{return Err(ObligationKind::EntryValueUnknown)};
            let read=need(&data.occurrences,request.access)?;let parameter=need(&data.occurrences,*declaration)?;let owner_row=need(&data.occurrences,*owner)?;
            if read.role!=OccurrenceRole::Read || parameter.role!=OccurrenceRole::Parameter || parameter.syntax_kind!=SyntaxKind::Parameter || !inside(read,owner_row) || !inside(parameter,owner_row) {return Err(ObligationKind::EntryValueUnknown);}
            for occurrence in [request.access] {let owner_row=data.owners.iter().find(|o|o.occurrence==occurrence).ok_or(ObligationKind::MissingEvidence)?;if owner_row.owner!=*owner || owner_row.entity!=request.owner{return Err(ObligationKind::EntryValueUnknown);}}
            let formal=PlaceRoot::Formal{declaration:*declaration};let place=Place{root:formal.id(),path:AccessPath::empty().id()};
            if data.roots.get(formal.id())!=Some(&formal) || data.paths.get(AccessPath::empty().id())!=Some(&AccessPath::empty()) || data.places.get(place.id())!=Some(&place){return Err(ObligationKind::MissingEvidence);}
            let mut links=data.links.iter().filter(|l|l.entity==request.formal && l.declaration.is_some());let link=links.next().ok_or(ObligationKind::MissingEvidence)?;if links.next().is_some(){return Err(ObligationKind::EntryValueUnknown);}
            let declared=need(&data.declarations,link.declaration.unwrap())?;if declared.parameter!=link.parameter || declared.declaration!=*declaration {return Err(ObligationKind::MissingEvidence);}exact(data,declared.qualification,request)?;
            let parameter_row=need(&data.parameters,link.parameter)?;let signature=need(&data.signatures,parameter_row.signature)?;let symbol=need(&data.symbols,signature.symbol)?;
            if symbol.context!=request.context || !matches!(symbol.kind,crate::domain::calls::SymbolKind::Function|crate::domain::calls::SymbolKind::Method){return Err(ObligationKind::MissingEvidence);}
            let mut declaration_support=None;
            for s in data.declaration_supports.iter().filter(|s|s.assertion==declared.id()) {let a=s.attribution().ok_or(ObligationKind::MissingEvidence)?;let native=need(&data.runs,a.run)?;if native.input==run.input && native.provider==symbol.provider && native.context==request.context {let frame=EntryRequest{run:a.run,..request};exact(data,declared.qualification,frame)?;if s.origin!=Origin::AnalyzerAssertion || s.mode!=ExtractionMode::NativeTraversal || !supported(data,s,frame)? {return Err(ObligationKind::MissingEvidence);}declaration_support=Some(s.id());}}
            let declaration_support=declaration_support.ok_or(ObligationKind::MissingEvidence)?;let mut owner_support=None;
            for d in data.symbol_declarations.iter().filter(|d|d.symbol==signature.symbol && data.qualifications.get(d.qualification).is_some_and(|q|q.context==request.context)) {if d.declaration!=*owner{return Err(ObligationKind::EntryValueUnknown);}for s in data.symbol_declaration_supports.iter().filter(|s|s.assertion==d.id()){let a=s.attribution().ok_or(ObligationKind::MissingEvidence)?;let native=need(&data.runs,a.run)?;if native.input==run.input && native.provider==symbol.provider && native.context==request.context {let frame=EntryRequest{run:a.run,..request};exact(data,d.qualification,frame)?;if s.origin!=Origin::AnalyzerAssertion || s.mode!=ExtractionMode::NativeTraversal || !supported(data,s,frame)?{return Err(ObligationKind::MissingEvidence);}owner_support=Some(s.id());}}}
            let owner_support=owner_support.ok_or(ObligationKind::MissingEvidence)?;
            let mut uses=data.uses.iter().filter(|u|u.occurrence==request.access && u.place==place.id());let use_=uses.next().ok_or(ObligationKind::MissingEvidence)?;if uses.next().is_some(){return Err(ObligationKind::EntryValueUnknown);}
            let mut observed=None;
            for row in data.use_observations.iter().filter(|o|o.use_==use_.id()) {let mut supports=data.use_supports.iter().filter(|s|s.assertion==row.id());let mut selected=None;for s in supports.by_ref(){if supported(data,s,request)? {if s.origin!=Origin::AnalyzerAssertion || s.mode!=ExtractionMode::NativeTraversal{return Err(ObligationKind::MissingEvidence);}selected=Some(s.id());}}if let Some(s)=selected {exact(data,row.qualification,request)?;if observed.is_some() || row.annotation{return Err(ObligationKind::EntryValueUnknown);}observed=Some((row,s));}}
            let (use_observation,use_support)=observed.ok_or(ObligationKind::MissingEvidence)?;let lexical=need(&data.lexical_scopes,use_observation.scope)?;
            if lexical.owner!=*owner || !matches!(lexical.kind,LexicalScopeKind::Function|LexicalScopeKind::Lambda){return Err(ObligationKind::EntryValueUnknown);}
            let source=source.cloned().unwrap_or(EntryAccessSource::Use{observation:use_observation.id(),support:use_support});
            let (access_q,region_q)=match &source {
             EntryAccessSource::Use{observation,support}=>{if *observation!=use_observation.id()||*support!=use_support{return Err(ObligationKind::MissingEvidence);}(use_observation.qualification,None)},
             EntryAccessSource::Value{observation,support,region,region_support}=>{let row=need(&data.values,*observation)?;let support=need(&data.value_supports,*support)?;let sink=need(&data.occurrences,row.sink)?;if row.use_!=use_.id()||!inside(sink,owner_row)||!data.owners.iter().any(|o|o.occurrence==row.sink&&o.entity==request.owner&&o.owner==*owner)||support.assertion!=row.id()||support.origin!=Origin::AnalyzerAssertion||support.mode!=ExtractionMode::NativeTraversal||!supported(data,support,request)?{return Err(ObligationKind::MissingEvidence);}if region_for(data,request)?!=(*region,*region_support){return Err(ObligationKind::MissingEvidence);}(row.qualification,Some(need(&data.regions,*region)?.qualification))},
             EntryAccessSource::Guard{observation,support,region,region_support}=>{let row=need(&data.leaves,*observation)?;let support=need(&data.leaf_supports,*support)?;let operand=need(&data.occurrences,row.operand.ok_or(ObligationKind::MissingEvidence)?)?;if operand.source!=read.source||operand.structural_path!=read.structural_path||operand.syntax_kind!=read.syntax_kind||operand.start!=read.start||operand.end!=read.end||!matches!(operand.role,OccurrenceRole::Syntax|OccurrenceRole::Read)||support.assertion!=row.id()||support.origin!=Origin::AnalyzerAssertion||support.mode!=ExtractionMode::NativeTraversal||!supported(data,support,request)?{return Err(ObligationKind::MissingEvidence);}let atom=need(&data.atoms,row.atom)?;let evaluation=need(&data.occurrences,atom.evaluation)?;let test=need(&data.occurrences,row.test)?;if atom.context!=request.context||atom.operand!=Some(use_.place)||!inside(read,evaluation)||!(test.id()==evaluation.id()||inside(evaluation,test)){return Err(ObligationKind::MissingEvidence);}if region_for(data,request)?!=(*region,*region_support){return Err(ObligationKind::MissingEvidence);}(row.qualification,Some(need(&data.regions,*region)?.qualification))},
            };
            let mut access_q=exact(data,access_q,request)?.clone();
            let bytes=data.condition_nodes.len().checked_mul(2048).ok_or(ObligationKind::ResourceRefused)?;
            let _buffer=budget.reserve("entry_condition_decode",bytes).map_err(|error|{resource_error=Some(error);ObligationKind::ResourceRefused})?;
            let nodes=data.condition_nodes.iter().cloned().collect::<Vec<_>>();
            let mut access_condition=super::Diagram::from_records(need(&data.conditions,access_q.condition)?,&nodes).map_err(|_|ObligationKind::MissingEvidence)?;
            if let Some(region_q)=region_q{let region_q=exact(data,region_q,request)?;let region_condition=super::Diagram::from_records(need(&data.conditions,region_q.condition)?,&nodes).map_err(|_|ObligationKind::MissingEvidence)?;if matches!(source,EntryAccessSource::Guard{..}){access_condition=region_condition;}else{let admitted=access_condition.admitted_binary(&region_condition,super::BooleanOperation::Conjunction,budget).map_err(|e|match e{super::DiagramAdmissionError::Resource(error)=>{resource_error=Some(error);ObligationKind::ResourceRefused},super::DiagramAdmissionError::Boundary(e)=>crate::domain::obligation::from_kernel(e)})?;let(result,reservation)=admitted.into_parts();charge.grow(result.allocation_allowance()).map_err(|error|{resource_error=Some(error);ObligationKind::ResourceRefused})?;access_condition=result;drop(reservation);}access_q.condition=access_condition.id();}
            charge.grow(access_condition.allocation_allowance()).map_err(|error|{resource_error=Some(error);ObligationKind::ResourceRefused})?;
            let mut selected=None;
            for row in data.reaching.iter().filter(|r|r.use_==use_.id()) {
                let q=need(&data.qualifications,row.qualification)?;if q.context!=request.context {continue;}
                let mut support_seen=false;let mut own=None;
                for s in data.reaching_supports.iter().filter(|s|s.assertion==row.id()){support_seen=true;if supported(data,s,request)? {if s.origin!=Origin::AnalyzerAssertion || s.mode!=ExtractionMode::NativeTraversal{return Err(ObligationKind::MissingEvidence);}own=Some(s.id());}}
                if !support_seen{return Err(ObligationKind::MissingEvidence);}
                if let Some(s)=own {exact(data,row.qualification,request)?;if selected.is_some() || row.loop_carried{return Err(ObligationKind::EntryValueUnknown);}selected=Some((row,s));}
            }
            let (reaching,reaching_support)=selected.ok_or(ObligationKind::EntryValueUnknown)?;
            let use_q=access_q;let reaching_q=need(&data.qualifications,reaching.qualification)?;
            if use_q.condition!=reaching_q.condition {
                let use_condition=&access_condition;
                let reaching_condition=super::Diagram::from_records(need(&data.conditions,reaching_q.condition)?,&nodes).map_err(|_|ObligationKind::MissingEvidence)?;
                let covered=use_condition.admitted_binary(&reaching_condition,super::BooleanOperation::Conjunction,budget).map_err(|e|match e {super::DiagramAdmissionError::Resource(error)=>{resource_error=Some(error);ObligationKind::ResourceRefused},super::DiagramAdmissionError::Boundary(e)=>crate::domain::obligation::from_kernel(e)})?;
                if covered.into_parts().0.id()!=use_condition.id(){return Err(ObligationKind::EntryValueUnknown);}
            }
            let ReachingDefinition::Bound{definition}=need(&data.targets,reaching.target)? else{return Err(ObligationKind::EntryValueUnknown)};
            let definition_row=need(&data.definitions,*definition)?;if definition_row.place!=place.id(){return Err(ObligationKind::EntryValueUnknown);}
            let parameter_placement=if definition_row.occurrence==*declaration{None}else{
                let identifier=need(&data.occurrences,definition_row.occurrence)?;
                if identifier.syntax_kind!=SyntaxKind::Identifier || identifier.role!=OccurrenceRole::Syntax || !inside(identifier,parameter) || identifier.structural_path.len()!=parameter.structural_path.len()+1 || identifier.start!=parameter.start{return Err(ObligationKind::EntryValueUnknown);}
                let mut selected=None;
                for row in data.placements.iter().filter(|p|p.occurrence==identifier.id() && p.parent==Some(parameter.id()) && p.field==SyntaxField::Child && p.ordinal==0){
                    for support in data.placement_supports.iter().filter(|s|s.assertion==row.id()){
                        let a=support.attribution().ok_or(ObligationKind::MissingEvidence)?;let native=need(&data.runs,a.run)?;
                        if native.input==run.input && native.context==request.context && native.provider==symbol.provider{let frame=EntryRequest{run:a.run,..request};exact(data,row.qualification,frame)?;if support.origin!=Origin::SourceObservation || support.mode!=ExtractionMode::NativeTraversal || !supported(data,support,frame)?{return Err(ObligationKind::MissingEvidence);}if selected.is_some(){return Err(ObligationKind::EntryValueUnknown);}selected=Some((row.id(),support.id()));}
                    }
                }
                Some(selected.ok_or(ObligationKind::MissingEvidence)?)
            };
            let mut definition_observation=None;
            for row in data.definition_observations.iter().filter(|o|o.definition==*definition) {let mut own=None;for s in data.definition_supports.iter().filter(|s|s.assertion==row.id()){if supported(data,s,request)?{if s.origin!=Origin::AnalyzerAssertion || s.mode!=ExtractionMode::NativeTraversal{return Err(ObligationKind::MissingEvidence);}own=Some(s.id());}}if let Some(s)=own{exact(data,row.qualification,request)?;if definition_observation.is_some() || row.scope!=use_observation.scope || row.kind!=BindingEventKind::Parameter || row.value.is_some(){return Err(ObligationKind::EntryValueUnknown);}definition_observation=Some((row,s));}}
            let (definition,definition_support)=definition_observation.ok_or(ObligationKind::EntryValueUnknown)?;
            let mut coverage=None;
            for row in data.coverage.iter().filter(|c|c.run==Some(request.run) && c.context==request.context && c.provider==Some(run.provider) && c.family==FactFamily::Flow) {
                let q=AssertionQualification{context:request.context,scope:row.scope,condition:need(&data.qualifications,use_observation.qualification)?.condition,modality:Modality::Definite,approximation:Approximation::Exact};
                let relevant=match data.scopes.get(q.scope){Some(CoverageScope::Input{input})=>*input==run.input,Some(CoverageScope::Artifact{artifact})=>*artifact==read.source,Some(CoverageScope::Module{module})=>data.modules.get(*module).is_some_and(|m|m.source==read.source),_=>false};
                if relevant {if row.status!=CoverageStatus::CompleteUnderStatedModel{return Err(ObligationKind::IncompleteCoverage);}coverage=Some(row.id());}
            }
            let coverage=coverage.ok_or(ObligationKind::IncompleteCoverage)?;
            use crate::domain::analysis::policy::{self,SupportRole,EvidenceStatus};
            let mut statuses=[(SupportRole::Support,EvidenceStatus::Unresolved);8];let mut count=0;let mut evidence=|family,fidelity|{statuses[count]=(SupportRole::Support,policy::native_status(family,fidelity));count+=1;};
            evidence(FactFamily::Flow,need(&data.use_supports,use_support)?.fidelity);evidence(FactFamily::Flow,need(&data.reaching_supports,reaching_support)?.fidelity);evidence(FactFamily::Flow,need(&data.definition_supports,definition_support)?.fidelity);evidence(FactFamily::Signatures,need(&data.declaration_supports,declaration_support)?.fidelity);evidence(FactFamily::Signatures,need(&data.symbol_declaration_supports,owner_support)?.fidelity);
            if let Some((_,support))=parameter_placement{evidence(FactFamily::Syntax,need(&data.placement_supports,support)?.fidelity);}
            match &source{EntryAccessSource::Use{..}=>{},EntryAccessSource::Value{support,region_support,..}=>{evidence(FactFamily::Flow,need(&data.value_supports,*support)?.fidelity);evidence(FactFamily::Flow,need(&data.region_supports,*region_support)?.fidelity);},EntryAccessSource::Guard{support,region_support,..}=>{evidence(FactFamily::Flow,need(&data.leaf_supports,*support)?.fidelity);evidence(FactFamily::Flow,need(&data.region_supports,*region_support)?.fidelity);}}
            let status=policy::derive_status(&statuses[..count]);
            let root=PlaceRoot::Entry{declaration:*declaration};let place=Place{root:root.id(),path:AccessPath::empty().id()};
            Ok(DerivedEntryValue{source:source.clone(),status,qualification:use_q,condition:access_condition,witness:Self{access_source:source.id(),owner:request.owner,formal:request.formal,access:request.access,context:request.context,run:request.run,link:link.id(),declaration_support,owner_support,use_observation:use_observation.id(),use_support,reaching:reaching.id(),reaching_support,definition:definition.id(),definition_support,parameter_placement:parameter_placement.map(|p|p.0),parameter_placement_support:parameter_placement.map(|p|p.1),coverage},parameter:link.parameter,root,place,_charge:std::sync::Arc::new(charge)})
        })();if let Some(error)=resource_error{Err(error)}else{Ok(result)}
    }
}
pub fn entry_invariants()->Vec<Invariant> {
    let mut inputs=EntryData::validation_inputs();inputs.extend([ValidationInput::of::<EntryValueWitness>(&["id"]),ValidationInput::of::<EntryAccessSource>(&["id"])]);
    vec![Invariant{name:"entry_value_witness_replay",inputs,create:std::sync::Arc::new(|budget|Box::new(EntryCheck{data:EntryData::new(budget),witnesses:Rows::new(budget),sources:Rows::new(budget),budget:budget.clone()}))}]
}
struct EntryCheck{data:EntryData,witnesses:Rows<EntryValueWitness>,sources:Rows<EntryAccessSource>,budget:resources::ResourceBudget}
impl InvariantCheck for EntryCheck {
    fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{if self.data.visit(name,batch)?{Ok(())}else if name==EntryValueWitness::NAME{self.witnesses.decode(batch)}else if name==EntryAccessSource::NAME{self.sources.decode(batch)}else{Err(ModelError::Invalid("undeclared entry witness input".into()))}}
    fn finish(self:Box<Self>)->Result<(),ModelError>{for stored in self.witnesses.iter(){let proof=EntryValueWitness::derive_for(&self.data,stored.request(),self.sources.get(stored.access_source).ok_or_else(||ModelError::Invalid("entry source absent".into()))?,&self.budget)?.map_err(|reason|ModelError::Invalid(format!("entry witness refused: {reason:?}")))?;if proof.witness()!=stored{return Err(ModelError::Invalid("stored entry witness differs from replay".into()));}}Ok(())}
}


use crate::{Domain,DomainSum};

/// Exact native execution-domain source; no producer-selected condition is accepted.
#[derive(Debug,Clone,PartialEq,Eq,Hash,DomainSum)]
#[model(name="entry_access_sources",rule="entry_access_source")]
pub enum EntryAccessSource {
 #[model(code=0)] Use {#[model(premise)] observation:Id<FlowUseObservation>,#[model(premise)] support:Id<FlowUseSupport>},
 #[model(code=1)] Value {#[model(premise)] observation:Id<FlowValueObservation>,#[model(premise)] support:Id<FlowValueSupport>,#[model(premise)] region:Id<FlowRegionObservation>,#[model(premise)] region_support:Id<FlowRegionSupport>},
 #[model(code=2)] Guard {#[model(premise)] observation:Id<FlowTestLeafObservation>,#[model(premise)] support:Id<FlowTestLeafSupport>,#[model(premise)] region:Id<FlowRegionObservation>,#[model(premise)] region_support:Id<FlowRegionSupport>},
}
/// Selection is structural and replayed; a source constructor supplies no proof authority.
impl EntryAccessSource {
 pub fn value(data:&EntryData,request:EntryRequest,observation:Id<FlowValueObservation>,support:Id<FlowValueSupport>)->Result<Self,ObligationKind>{let(region,region_support)=region_for(data,request)?;Ok(Self::Value{observation,support,region,region_support})}
 pub fn guard(data:&EntryData,request:EntryRequest,observation:Id<FlowTestLeafObservation>,support:Id<FlowTestLeafSupport>)->Result<Self,ObligationKind>{let(region,region_support)=region_for(data,request)?;Ok(Self::Guard{observation,support,region,region_support})}
}
fn region_for(data:&EntryData,request:EntryRequest)->Result<(Id<FlowRegionObservation>,Id<FlowRegionSupport>),ObligationKind>{region_for_access(data,request.owner,request.access,request.context,request.run)}
/// Exact containing native execution region; parameter identity remains a separate Entry operation.
pub(crate) fn region_for_access(data:&EntryData,owner:Id<EntityRef>,access:Id<Occurrence>,context:Id<AnalysisContext>,run:Id<ProviderRun>)->Result<(Id<FlowRegionObservation>,Id<FlowRegionSupport>),ObligationKind>{
let request=AccessFrame{owner,access,context,run};

 let read=need(&data.occurrences,request.access)?;
 let mut selected=None;
 for row in data.use_observations.iter().filter(|o|data.uses.get(o.use_).is_some_and(|u|u.occurrence==request.access)&&data.qualifications.get(o.qualification).is_some_and(|q|q.context==request.context)){for support in data.use_supports.iter().filter(|s|s.assertion==row.id()){if supported(data,support,request)?{if selected.is_some(){return Err(ObligationKind::MissingEvidence);}selected=Some(row);}}}
 let observed=selected.ok_or(ObligationKind::MissingEvidence)?;
 let scope=need(&data.lexical_scopes,observed.scope)?;
 let mut best:Option<&FlowRegionObservation>=None;
 for row in data.regions.iter().filter(|r|r.scope==observed.scope&&data.qualifications.get(r.qualification).is_some_and(|q|q.context==request.context)){
  let statement=need(&data.occurrences,row.statement)?;
  if !inside(read,statement)||!inside(statement,need(&data.occurrences,scope.owner)?){continue;}
  if !data.owners.iter().any(|o|o.occurrence==statement.id()&&o.owner==scope.owner&&o.entity==request.owner){return Err(ObligationKind::EntryValueUnknown);}
  match best{Some(previous)=>{let depth=need(&data.occurrences,previous.statement)?.structural_path.len();if statement.structural_path.len()==depth{return Err(ObligationKind::MissingEvidence);}if statement.structural_path.len()>depth{best=Some(row);}},None=>best=Some(row)}
 }
 let row=best.ok_or(ObligationKind::MissingEvidence)?;exact(data,row.qualification,request)?;
 let mut selected=None;for support in data.region_supports.iter().filter(|s|s.assertion==row.id()){if supported(data,support,request)?{if support.origin!=Origin::AnalyzerAssertion||support.mode!=ExtractionMode::NativeTraversal||selected.is_some(){return Err(ObligationKind::MissingEvidence);}selected=Some(support.id());}}
 Ok((row.id(),selected.ok_or(ObligationKind::MissingEvidence)?))
}
