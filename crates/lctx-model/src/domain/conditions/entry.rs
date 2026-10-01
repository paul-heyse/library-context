//! Shared proof of a source parameter binding's entry value at one exact read.
//! This proves binding identity, never mutable object-state stability.
use crate::domain::{*, assertion::*, attribution::*, declarations::*, flow::*, lexical::*, normalized::{Rows,entities::*}, source::*, syntax::{SyntaxPlacement,SyntaxPlacementSupport}, value::*};
use crate::Domain;

#[macro_export]
macro_rules! entry_value_inputs {($apply:ident)=>{$apply! {
    artifacts: $crate::domain::source::SourceArtifact,
    modules: $crate::domain::source::Module,
    scopes: $crate::domain::source::CoverageScope,
    occurrences: $crate::domain::source::Occurrence,
    qualifications: $crate::domain::assertion::AssertionQualification,
    predicates: $crate::domain::value::Predicate,
    atoms: $crate::domain::conditions::EvaluationAtom,
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
pub struct DerivedEntryValue {witness:EntryValueWitness,parameter:Id<crate::domain::calls::SignatureParameter>,root:PlaceRoot,place:Place,_charge:std::sync::Arc<charged::StateCharge>}
impl DerivedEntryValue {
    pub fn witness(&self)->&EntryValueWitness {&self.witness}
    pub fn parameter(&self)->Id<crate::domain::calls::SignatureParameter> {self.parameter}
    pub fn root(&self)->&PlaceRoot {&self.root}
    pub fn place(&self)->&Place {&self.place}
    pub(crate) fn allowance(&self)->std::sync::Arc<charged::StateCharge>{self._charge.clone()}
}
fn need<R:Record>(rows:&Rows<R>,id:Id<R>)->Result<&R,ObligationKind> {rows.get(id).ok_or(ObligationKind::MissingEvidence)}
fn inside(child:&Occurrence,parent:&Occurrence)->bool {child.source==parent.source && child.structural_path.len()>parent.structural_path.len() && child.structural_path.starts_with(&parent.structural_path) && child.start>=parent.start && child.end<=parent.end}
fn exact(data:&EntryData,id:Id<AssertionQualification>,request:EntryRequest)->Result<&AssertionQualification,ObligationKind> {
    let q=need(&data.qualifications,id)?;
    if q.context!=request.context {return Err(ObligationKind::IncompatibleContexts);}
    if q.modality!=Modality::Definite || q.approximation!=Approximation::Exact {return Err(ObligationKind::Approximation);}
    let input=need(&data.runs,request.run)?.input;
    let source=need(&data.occurrences,request.access)?.source;
    let artifact=need(&data.artifacts,source)?;
    let covered=match need(&data.scopes,q.scope)? {CoverageScope::Input{input:owner}=>*owner==input && artifact.input==input,CoverageScope::Artifact{artifact:owner}=>*owner==source && artifact.input==input,CoverageScope::Module{module}=>need(&data.modules,*module)?.source==source && artifact.input==input,_=>false};
    if !covered {return Err(ObligationKind::MissingEvidence);}Ok(q)
}
fn supported<S:Support>(data:&EntryData,support:&S,request:EntryRequest)->Result<bool,ObligationKind> {
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
        let mut charge=charged::StateCharge::new(budget,"entry-value-derivation");charge.grow(512)?;
        Ok((||{
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
            let mut selected=None;
            for row in data.reaching.iter().filter(|r|r.use_==use_.id()) {
                let q=need(&data.qualifications,row.qualification)?;if q.context!=request.context {continue;}
                let mut support_seen=false;let mut own=None;
                for s in data.reaching_supports.iter().filter(|s|s.assertion==row.id()){support_seen=true;if supported(data,s,request)? {if s.origin!=Origin::AnalyzerAssertion || s.mode!=ExtractionMode::NativeTraversal{return Err(ObligationKind::MissingEvidence);}own=Some(s.id());}}
                if !support_seen{return Err(ObligationKind::MissingEvidence);}
                if let Some(s)=own {exact(data,row.qualification,request)?;if selected.is_some() || row.loop_carried || q.condition!=need(&data.qualifications,use_observation.qualification)?.condition{return Err(ObligationKind::EntryValueUnknown);}selected=Some((row,s));}
            }
            let (reaching,reaching_support)=selected.ok_or(ObligationKind::EntryValueUnknown)?;
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
            let root=PlaceRoot::Entry{declaration:*declaration};let place=Place{root:root.id(),path:AccessPath::empty().id()};
            Ok(DerivedEntryValue{witness:Self{owner:request.owner,formal:request.formal,access:request.access,context:request.context,run:request.run,link:link.id(),declaration_support,owner_support,use_observation:use_observation.id(),use_support,reaching:reaching.id(),reaching_support,definition:definition.id(),definition_support,parameter_placement:parameter_placement.map(|p|p.0),parameter_placement_support:parameter_placement.map(|p|p.1),coverage},parameter:link.parameter,root,place,_charge:std::sync::Arc::new(charge)})
        })())
    }
}
pub fn entry_invariants()->Vec<Invariant> {
    let mut inputs=EntryData::validation_inputs();inputs.push(ValidationInput::of::<EntryValueWitness>(&["id"]));
    vec![Invariant{name:"entry_value_witness_replay",inputs,create:std::sync::Arc::new(|budget|Box::new(EntryCheck{data:EntryData::new(budget),witnesses:Rows::new(budget),budget:budget.clone()}))}]
}
struct EntryCheck{data:EntryData,witnesses:Rows<EntryValueWitness>,budget:resources::ResourceBudget}
impl InvariantCheck for EntryCheck {
    fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<(),ModelError>{if self.data.visit(name,batch)?{Ok(())}else if name==EntryValueWitness::NAME{self.witnesses.decode(batch)}else{Err(ModelError::Invalid("undeclared entry witness input".into()))}}
    fn finish(self:Box<Self>)->Result<(),ModelError>{for stored in self.witnesses.iter(){let proof=EntryValueWitness::derive(&self.data,stored.request(),&self.budget)?.map_err(|reason|ModelError::Invalid(format!("entry witness refused: {reason:?}")))?;if proof.witness()!=stored{return Err(ModelError::Invalid("stored entry witness differs from replay".into()));}}Ok(())}
}
