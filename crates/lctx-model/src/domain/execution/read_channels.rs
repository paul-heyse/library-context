//! Native read/dependency observations and complete-negative assessments. Read existence is
//! distinct from execution reachability and value availability. No heap state is inferred.
use super::evaluation::EvaluationData;
use crate::domain::{
    analysis::{
        base_evaluation as publication, native::NativeAssertionPremise, policy::EvidenceStatus,
    },
    assertion::*,
    attribution::*,
    conditions::entry::EntryData,
    flow::*,
    normalized::{Rows, entities::*},
    source::*,
    value::*,
    *,
};
use crate::{Domain, DomainCode};
#[derive(Debug, Clone, Copy, PartialEq, Eq, DomainCode)]
#[repr(i16)]
pub enum ReadLocation {
    NativePlace = 0,
    DeclaredGlobal = 1,
    UnresolvedGlobal = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_read_observations")]
pub struct ReadObservation {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub observation: Id<FlowUseObservation>,
    pub owner: Id<EntityRef>,
    pub place: Id<Place>,
    pub location: ReadLocation,
    pub lexical_premise: Option<Id<NativeAssertionPremise>>,
    pub binding_premise: Option<Id<NativeAssertionPremise>>,
    pub qualification: Id<AssertionQualification>,
    pub premise: Option<Id<NativeAssertionPremise>>,
    pub status: EvidenceStatus,
    pub execution_region: Option<Id<FlowRegionObservation>>,
    pub region_premise: Option<Id<NativeAssertionPremise>>,
    pub reason: Option<obligation::ObligationKind>,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_read_dependencies")]
pub struct ReadDependency {
    #[model(key)]
    pub read: Id<ReadObservation>,
    #[model(key)]
    pub observation: Id<FlowValueObservation>,
    pub premise: Option<Id<NativeAssertionPremise>>,
    pub qualification: Id<AssertionQualification>,
    pub status: EvidenceStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_attribute_reads")]
pub struct AttributeRead {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub observation: Id<FlowAttributeLoadObservation>,
    pub owner: Id<EntityRef>,
    pub premise: Option<Id<NativeAssertionPremise>>,
    pub qualification: Id<AssertionQualification>,
    pub status: EvidenceStatus,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, DomainCode)]
#[repr(i16)]
pub enum ReadAssessment {
    ObservedRead = 0,
    CompleteNoReadUnderModel = 1,
    Unknown = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "base_formal_read_assessments")]
pub struct FormalReadAssessment {
    #[model(key)]
    pub invocation: Id<publication::AnalysisInvocation>,
    #[model(key)]
    pub formal: Id<ParameterEntity>,
    pub owner: Id<EntityRef>,
    pub scope: Option<Id<lexical::LexicalScope>>,
    pub coverage: Option<Id<ProviderCoverage>>,
    pub status: ReadAssessment,
    pub reason: Option<obligation::ObligationKind>,
    pub reads: ContentHash,
}
pub struct ReadRecords {
    pub fields: super::read_fields::FieldRecords,
    pub dynamic: Rows<super::read_dynamic::DynamicAccessObservation>,
    pub dynamic_premises: Rows<super::read_dynamic::DynamicAccessPremise>,
    pub reads: Rows<ReadObservation>,
    pub dependencies: Rows<ReadDependency>,
    pub attributes: Rows<AttributeRead>,
    pub formals: Rows<FormalReadAssessment>,
}
impl ReadRecords {
    pub fn new(budget: &resources::ResourceBudget) -> Self {
        Self {
            fields: super::read_fields::FieldRecords::new(budget),
            dynamic: Rows::new(budget),
            dynamic_premises: Rows::new(budget),
            reads: Rows::new(budget),
            dependencies: Rows::new(budget),
            attributes: Rows::new(budget),
            formals: Rows::new(budget),
        }
    }
}
pub fn relations() -> Vec<Relation> {
    {
        let mut rows = super::read_dynamic::relations();
        rows.extend(super::read_fields::relations());
        rows.extend([
            Relation::of::<ReadObservation>(),
            Relation::of::<ReadDependency>(),
            Relation::of::<AttributeRead>(),
            Relation::of::<FormalReadAssessment>(),
        ]);
        rows
    }
}
impl ReadRecords {
    pub fn visit(
        &mut self,
        name: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.fields.visit(name, batch)?;
        macro_rules! rows{($($field:ident:$ty:ty,)*)=>{$(if name==<$ty>::NAME{self.$field.decode(batch)?;})*};}
        rows! {dynamic:super::read_dynamic::DynamicAccessObservation,dynamic_premises:super::read_dynamic::DynamicAccessPremise,reads:ReadObservation,dependencies:ReadDependency,attributes:AttributeRead,formals:FormalReadAssessment,}
        Ok(())
    }
    pub fn append(&mut self, other: &Self) -> Result<(), ModelError> {
        self.fields.append(&other.fields)?;
        macro_rules! rows{($($field:ident,)*)=>{$(for row in other.$field.iter(){self.$field.insert(row.clone())?;})*};}
        rows! {dynamic,dynamic_premises,reads,dependencies,attributes,formals,}
        Ok(())
    }
    pub fn same(&self, other: &Self) -> bool {
        self.fields.same(&other.fields)
            && self.dynamic.same(&other.dynamic)
            && self.dynamic_premises.same(&other.dynamic_premises)
            && self.reads.same(&other.reads)
            && self.dependencies.same(&other.dependencies)
            && self.attributes.same(&other.attributes)
            && self.formals.same(&other.formals)
    }
}
pub fn validation_inputs() -> Vec<ValidationInput> {
    {
        let mut inputs = super::read_dynamic::validation_inputs();
        inputs.extend(super::read_fields::validation_inputs());
        inputs.extend([
            ValidationInput::of::<ReadObservation>(&["id"]),
            ValidationInput::of::<ReadDependency>(&["id"]),
            ValidationInput::of::<AttributeRead>(&["id"]),
            ValidationInput::of::<FormalReadAssessment>(&["id"]),
        ]);
        inputs
    }
}
pub const WORK_LIMIT: usize = 1 << 24;
pub(super) struct Work {
    pub(super) dynamic_targets: charged::ChargedMap<Id<Occurrence>, Vec<Id<calls::CallTarget>>>,
    remaining: usize,
    native: charged::ChargedMap<
        (derivation::RowRef, derivation::RowRef),
        Id<analysis::native::NativeQualification>,
    >,
    native_ids:
        charged::ChargedMap<Id<NativeAssertionPremise>, Id<analysis::native::NativeQualification>>,
    owners: charged::ChargedMap<Id<Occurrence>, Option<Id<EntityRef>>>,
    formal_reads: charged::ChargedMap<Id<Occurrence>, Vec<Id<FlowUse>>>,
    nested: charged::ChargedMap<Id<EntityRef>, Vec<Id<FlowReachingObservation>>>,
    calls: charged::ChargedMap<Id<EntityRef>, Vec<Id<Occurrence>>>,
    charge: charged::StateCharge,
}
impl Work {
    pub(super) fn scan(&mut self, n: usize) -> Result<(), ModelError> {
        self.remaining = self
            .remaining
            .checked_sub(n)
            .ok_or_else(|| ModelError::Invalid("read channel work bound exhausted".into()))?;
        Ok(())
    }
    pub(super) fn tick(&mut self) -> Result<(), ModelError> {
        self.scan(1)
    }
    pub(super) fn owner(&self, site: Id<Occurrence>) -> Option<Id<EntityRef>> {
        self.owners.get(&site).copied().flatten()
    }
    pub(super) fn qualification<'a>(
        &self,
        data: &'a EvaluationData,
        premise: Id<NativeAssertionPremise>,
    ) -> Option<&'a analysis::native::NativeQualification> {
        data.native.get(*self.native_ids.get(&premise)?)
    }
    fn empty(budget:&resources::ResourceBudget)->Self {
        Self {
            dynamic_targets: Default::default(),
            remaining: WORK_LIMIT,
            native: Default::default(),
            native_ids: Default::default(),
            owners: Default::default(),
            formal_reads: Default::default(),
            nested: Default::default(),
            calls: Default::default(),
            charge: charged::StateCharge::new(budget, "read-channel-index"),
        }
    }
    fn new(
        data: &EvaluationData,
        entry: &EntryData,
        invocation: &publication::AnalysisInvocation,
        budget: &resources::ResourceBudget,
    ) -> Result<Self, ModelError> {
        let mut index = Self::empty(budget);
        for row in data.native.iter() {
            index.tick()?;
            let premise = data
                .premises
                .get(row.premise)
                .ok_or_else(|| ModelError::Invalid("read native pair missing".into()))?;
            if index
                .native
                .insert(&mut index.charge, premise.assertion_and_support(), row.id())?
                .is_some()
            {
                return Err(ModelError::Invalid("duplicate read native pair".into()));
            }
            index
                .native_ids
                .insert(&mut index.charge, row.premise, row.id())?;
        }
        for row in entry.owners.iter() {
            index.tick()?;
            let value = if index.owners.contains_key(&row.occurrence) {
                None
            } else {
                Some(row.entity)
            };
            index
                .owners
                .insert(&mut index.charge, row.occurrence, value)?;
        }
        let mut observed = charged::ChargedSet::default();
        for row in entry.use_observations.iter() {
            index.tick()?;
            if entry
                .qualifications
                .get(row.qualification)
                .is_some_and(|q| q.context == invocation.context)
            {
                observed.insert(&mut index.charge, row.use_)?;
            }
        }
        for row in entry.uses.iter() {
            index.tick()?;
            if !observed.contains(&row.id()) {
                continue;
            }
            if let Some(PlaceRoot::Formal { declaration }) = entry
                .places
                .get(row.place)
                .and_then(|p| entry.roots.get(p.root))
            {
                index
                    .formal_reads
                    .update(&mut index.charge, *declaration, |rows| rows.push(row.id()))?;
            }
        }
        let mut parameters = charged::ChargedMap::default();
        for row in entry.placements.iter() {
            index.tick()?;
            if row.field == lexical::SyntaxField::Child
                && row.ordinal == 0
                && let Some(parameter) = row.parent.filter(|p| {
                    entry
                        .occurrences
                        .get(*p)
                        .is_some_and(|o| o.syntax_kind == SyntaxKind::Parameter)
                })
            {
                parameters.insert(&mut index.charge, row.occurrence, parameter)?;
            }
        }
        for row in entry.reaching.iter() {
            index.tick()?;
            if entry
                .qualifications
                .get(row.qualification)
                .is_none_or(|q| q.context != invocation.context)
            {
                continue;
            }
            if let Some(ReachingDefinition::Bound { definition }) = entry.targets.get(row.target)
                && let Some(definition) = entry.definitions.get(*definition)
            {
                let parameter = parameters
                    .get(&definition.occurrence)
                    .copied()
                    .unwrap_or(definition.occurrence);
                index
                    .formal_reads
                    .update(&mut index.charge, parameter, |rows| rows.push(row.use_))?;
            }
            if matches!(
                entry.targets.get(row.target),
                Some(ReachingDefinition::Nested)
            ) && let Some(owner) = entry
                .uses
                .get(row.use_)
                .and_then(|u| index.owner(u.occurrence))
            {
                index
                    .nested
                    .update(&mut index.charge, owner, |rows| rows.push(row.id()))?;
            }
        }
        for target in data.call_targets.iter() {
            index.tick()?;
            if entry
                .qualifications
                .get(target.qualification)
                .is_some_and(|q| q.context == invocation.context)
            {
                index
                    .dynamic_targets
                    .update(&mut index.charge, target.site, |rows| {
                        rows.push(target.id())
                    })?;
            }
        }
        for occurrence in entry.occurrences.iter() {
            index.tick()?;
            if occurrence.syntax_kind == SyntaxKind::ExprCall
                && let Some(owner) = index.owner(occurrence.id())
            {
                index
                    .calls
                    .update(&mut index.charge, owner, |rows| rows.push(occurrence.id()))?;
            }
        }
        Ok(index)
    }
}
/// Fixed-value projections prepared from the complete immutable invocation inventory. The
/// compiler may join and stream these keys before decoding a rich read kernel. Preparation
/// retains no occurrence paths, literal bodies, native reports, or checked Local values.
pub enum ReadMetadata {
    Native {pair:(derivation::RowRef,derivation::RowRef),premise:Id<NativeAssertionPremise>,qualification:Id<analysis::native::NativeQualification>},
    Owner {occurrence:Id<Occurrence>,entity:Id<EntityRef>},
    FormalUse {parameter:Id<Occurrence>,use_:Id<FlowUse>},
    Nested {owner:Id<EntityRef>,reaching:Id<FlowReachingObservation>},
    CallTarget {site:Id<Occurrence>,target:Id<calls::CallTarget>},
    Call {owner:Id<EntityRef>,occurrence:Id<Occurrence>},
}
/// Actual Local entry values needed by a selected dynamic receiver inspection. Stored rows
/// select an admitted use receipt; the actual Local issuer supplies its value without replay.
#[derive(Clone,Copy)]
pub struct ReadEntries<'a> {
    pub witnesses:&'a Rows<conditions::entry::EntryValueWitness>,
    pub sources:&'a Rows<conditions::entry::EntryAccessSource>,
    pub actual:&'a local_semantics::ProducedLocal,
}
/// One complete semantic root, selected before rich decoding. Ancillary candidates in its
/// dependency closure cannot become additional output roots.
pub enum ReadRoot {
    Use(Id<FlowUseObservation>),Attribute(Id<flow::FlowAttributeLoadObservation>),
    DynamicCall(Id<calls::CallSyntax>),DynamicAttribute(Id<flow::FlowAttributeLoadObservation>),
    Formal(Id<ParameterEntity>),FieldCall(Id<calls::CallSyntax>),
    FieldClass(Id<syntax::ClassFieldSyntaxObservation>),FieldStore(Id<FlowDefinitionObservation>),
    FieldGlobal(Id<lexical::BindingObservation>),FieldAssessment {class:Id<ClassEntity>,name:String},
}
/// Invocation-local shared read work and complete-negative metadata. This is an execution
/// index, not a proof token or an independently admitted public authority.
pub struct PreparedReads {
    invocation:publication::AnalysisInvocation,
    work:Work,
    fields:super::read_fields::FieldUniverse,
}
impl PreparedReads {
    pub fn new(invocation:&publication::AnalysisInvocation,budget:&resources::ResourceBudget)->Self {
        Self {invocation:invocation.clone(),work:Work::empty(budget),fields:super::read_fields::FieldUniverse::new(budget)}
    }
    /// Charge the complete source inventory visits, including rows that do not enter an index.
    /// This preserves one owner work bound across preparation and every subsequent root.
    pub fn scan_metadata(&mut self,count:usize)->Result<(),ModelError> {self.work.scan(count)}
    pub fn prepare(&mut self,row:ReadMetadata)->Result<(),ModelError> {
        let work=&mut self.work;
        match row {
            ReadMetadata::Native {pair,premise,qualification}=>{
                if work.native.insert(&mut work.charge,pair,qualification)?.is_some(){return Err(ModelError::Invalid("duplicate read native pair".into()));}
                work.native_ids.insert(&mut work.charge,premise,qualification)?;
            }
            ReadMetadata::Owner {occurrence,entity}=>{let value=if work.owners.contains_key(&occurrence){None}else{Some(entity)};work.owners.insert(&mut work.charge,occurrence,value)?;}
            ReadMetadata::FormalUse {parameter,use_}=>{work.formal_reads.update(&mut work.charge,parameter,|rows|rows.push(use_))?;}
            ReadMetadata::Nested {owner,reaching}=>{work.nested.update(&mut work.charge,owner,|rows|rows.push(reaching))?;}
            ReadMetadata::CallTarget {site,target}=>{work.dynamic_targets.update(&mut work.charge,site,|rows|rows.push(target))?;}
            ReadMetadata::Call {owner,occurrence}=>{work.calls.update(&mut work.charge,owner,|rows|rows.push(occurrence))?;}
        }
        Ok(())
    }
    fn frame(&self,invocation:&publication::AnalysisInvocation)->Result<(),ModelError> {if self.invocation!=*invocation {Err(ModelError::Conflict("prepared read invocation changed"))}else{Ok(())}}
    pub fn produce(&mut self,data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,roots:&std::collections::BTreeSet<Id<SourceArtifact>>,root:ReadRoot,budget:&resources::ResourceBudget,actual:ReadEntries<'_>)->Result<ReadRecords,ModelError> {
        self.frame(invocation)?;
        let mut out=ReadRecords::new(budget);
        match root {
            ReadRoot::Use(id)=>produce_uses(data,entry,invocation,roots,&mut out,&mut self.work,Some(id))?,
            ReadRoot::Attribute(id)=>produce_attributes(data,entry,invocation,roots,&mut out,&mut self.work,Some(id))?,
            ReadRoot::DynamicCall(id)=>super::read_dynamic::produce_selected(data,entry,invocation,roots,&mut out,budget,&mut self.work,Some(super::read_dynamic::SelectedRoot::Call(id)),Some(actual))?,
            ReadRoot::DynamicAttribute(id)=>super::read_dynamic::produce_selected(data,entry,invocation,roots,&mut out,budget,&mut self.work,Some(super::read_dynamic::SelectedRoot::Attribute(id)),Some(actual))?,
            ReadRoot::Formal(id)=>produce_formals(data,entry,invocation,roots,&mut out,budget,&mut self.work,Some(id),self.fields.dynamic_scope())?,
            ReadRoot::FieldCall(id)=>self.fields.calls(data,entry,invocation,roots,&mut out,budget,&mut self.work,Some(id))?,
            ReadRoot::FieldClass(id)=>self.fields.classes(data,entry,invocation,roots,&mut out,budget,&mut self.work,Some(id))?,
            ReadRoot::FieldStore(id)=>self.fields.stores(data,entry,invocation,roots,&mut out,budget,&mut self.work,Some(id))?,
            ReadRoot::FieldGlobal(id)=>self.fields.globals(data,entry,invocation,roots,&mut out,budget,&mut self.work,Some(id),Some(actual))?,
            ReadRoot::FieldAssessment {class,name}=>self.fields.assess(data,entry,invocation,&class,&name,&mut out,budget,&mut self.work)?,
        }
        for row in out.attributes.iter(){let native=data.attribute_loads.get(row.observation).ok_or_else(||ModelError::Invalid("field read observation missing".into()))?;self.fields.remember_attribute(row,&native.name)?;}
        for row in out.dynamic.iter(){self.fields.remember_dynamic(row)?;}
        Ok(out)
    }
    /// Call once for each admitted artifact in canonical ID order, then visit attribute IDs.
    pub fn field_roots(&mut self,data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,roots:&std::collections::BTreeSet<Id<SourceArtifact>>)->Result<(),ModelError> {
        self.frame(invocation)?;self.fields.roots(data,entry,invocation,roots,&mut self.work)
    }
    pub fn begin_field_candidates(&mut self)->Result<(),ModelError> {self.fields.attributes(&mut self.work)}
    pub fn seal_fields(&mut self)->Result<(),ModelError> {self.fields.seal(&mut self.work)}
    pub fn field_keys(&self)->impl Iterator<Item=&(Id<ClassEntity>,String)> {self.fields.keys()}
    pub fn dynamic_classes(&self)->impl Iterator<Item=Id<ClassEntity>>+'_ {self.fields.dynamic_classes()}
}
pub(super) fn input_scope(
    entry: &EntryData,
    scope: Id<CoverageScope>,
    input: Id<input::InputRevision>,
    artifact: Id<SourceArtifact>,
) -> bool {
    match entry.scopes.get(scope) {
        Some(CoverageScope::Input { input: owner }) => *owner == input,
        Some(CoverageScope::Artifact { artifact: owner }) => *owner == artifact,
        Some(CoverageScope::Module { module }) => entry
            .modules
            .get(*module)
            .is_some_and(|m| m.source == artifact),
        _ => false,
    }
}
/// Native Ruff owns binding identity. Recognizer BindingObservation rows remain retained
/// characterization and cannot replace the exact attached native event/scope proof.
pub(super) fn native_binding<'a>(
    data: &'a EvaluationData,
    entry: &EntryData,
    invocation: &publication::AnalysisInvocation,
    event: Id<lexical::BindingEvent>,
    work: &mut Work,
) -> Result<Option<(&'a ruff::RuffBindingObservation, NativeSupport)>, ModelError> {
    let Some(event) = data.binding_events.get(event) else {
        return Ok(None);
    };
    let Some(site) = entry.occurrences.get(event.site) else {
        return Ok(None);
    };
    let mut found = None;
    work.scan(data.ruff_bindings.len())?;
    for binding in data.ruff_bindings.iter().filter(|b| b.event == event.id()) {
        work.tick()?;
        let Some(scope) = binding.scope.and_then(|s| entry.lexical_scopes.get(s)) else {
            continue;
        };
        let Some(owner) = entry.occurrences.get(scope.owner) else {
            continue;
        };
        let Some(q) = entry.qualifications.get(binding.qualification) else {
            continue;
        };
        if binding.scope_location != ruff::AttachmentStatus::Located
            || binding.native_name != event.name
            || q.context != invocation.context
            || q.modality != Modality::Definite
            || q.approximation != Approximation::Exact
            || site.source != owner.source
            || !site.structural_path.starts_with(&owner.structural_path)
        {
            continue;
        }
        let Some(proof) = native(
            NativeContext {
                data,
                entry,
                invocation,
            },
            &data.ruff_binding_supports,
            binding.id(),
            binding.qualification,
            event.site,
            work,
        )?
        else {
            continue;
        };
        if found.is_some() {
            return Ok(None);
        }
        found = Some((binding, proof));
    }
    Ok(found)
}
/// Calls may retain unresolved alternatives (Partial); that is not a runtime closure claim.
/// Missing/unavailable inventory cannot screen imported/qualified name-driven accesses.
pub(super) fn calls_available(
    entry: &EntryData,
    invocation: &publication::AnalysisInvocation,
    artifact: Id<SourceArtifact>,
) -> bool {
    entry.coverage.iter().any(|c| {
        c.family == FactFamily::Calls
            && c.context == invocation.context
            && input_scope(entry, c.scope, invocation.input, artifact)
            && matches!(
                c.status,
                CoverageStatus::CompleteUnderStatedModel | CoverageStatus::Partial
            )
            && c.run.is_some_and(|run| {
                entry
                    .runs
                    .get(run)
                    .is_some_and(|r| r.input == invocation.input && r.context == invocation.context)
            })
    })
}
pub(super) fn selected(
    entry: &EntryData,
    invocation: &publication::AnalysisInvocation,
    occurrence: Id<Occurrence>,
    roots: &std::collections::BTreeSet<Id<SourceArtifact>>,
) -> bool {
    entry.occurrences.get(occurrence).is_some_and(|row| {
        roots.contains(&row.source)
            && entry
                .artifacts
                .get(row.source)
                .is_some_and(|a| a.input == invocation.input)
    })
}
pub(super) type NativeSupport = (Id<NativeAssertionPremise>, Id<ProviderRun>, EvidenceStatus);

pub(super) struct NativeContext<'a> {
    pub(super) data: &'a EvaluationData,
    pub(super) entry: &'a EntryData,
    pub(super) invocation: &'a publication::AnalysisInvocation,
}

// The same assertion can retain recognizer and native supports. This operation admits the
// authority of its own question; retained characterization is not a second native answer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum NativeQuestion {
    ExecutableRead,
    DeclaredClassInspection,
    SourceClassHierarchy,
}
fn native_authority(
    attribution: &SupportAttribution,
    premise: &NativeAssertionPremise,
    qualification: &analysis::native::NativeQualification,
    question: NativeQuestion,
) -> bool {
    use NativeAssertionPremise as P;
    if question == NativeQuestion::SourceClassHierarchy
        && !matches!(premise, P::ClassAncestryObservation { .. })
    {
        return false;
    }
    // These are source-syntax questions, including CallSyntax used by read_dynamic.
    // Parameter declarations and function traits remain analyzer questions even though
    // their families can also contain canonical parameter syntax.
    let source_syntax = matches!(
        premise,
        P::SyntaxObservation { .. }
            | P::SyntaxPlacement { .. }
            | P::CallSyntax { .. }
            | P::DeclarationObservation { .. }
            | P::SyntaxDetailObservation { .. }
            | P::ParameterSyntaxObservation { .. }
            | P::ClassFieldSyntaxObservation { .. }
    );
    let report_inspection = match question {
        NativeQuestion::DeclaredClassInspection => matches!(
            premise,
            P::FunctionTraitObservation { .. }
                | P::SymbolDeclaration { .. }
                | P::ParameterDeclaration { .. }
        ),
        NativeQuestion::SourceClassHierarchy => {
            matches!(premise, P::ClassAncestryObservation { .. })
        }
        NativeQuestion::ExecutableRead => false,
    } && attribution.origin == Origin::AnalyzerAssertion
        && attribution.fidelity == Fidelity::ReportProjection;
    attribution.mode == ExtractionMode::NativeTraversal
        && attribution.fidelity == qualification.fidelity
        && (report_inspection
            || if source_syntax {
                attribution.origin == Origin::SourceObservation
                    && matches!(
                        attribution.fidelity,
                        Fidelity::Raw | Fidelity::NativeStructural
                    )
            } else {
                attribution.origin == Origin::AnalyzerAssertion
                    && attribution.fidelity == Fidelity::NativeStructural
            })
}

pub(super) fn native<S: Support>(
    native_context: NativeContext<'_>,
    supports: &Rows<S>,
    assertion: Id<S::Assertion>,
    q: Id<AssertionQualification>,
    site: Id<Occurrence>,
    work: &mut Work,
) -> Result<Option<NativeSupport>, ModelError> {
    native_for(
        native_context,
        supports,
        assertion,
        q,
        site,
        work,
        NativeQuestion::ExecutableRead,
    )
}
pub(super) fn declared_class_inspection<S: Support>(
    native_context: NativeContext<'_>,
    supports: &Rows<S>,
    assertion: Id<S::Assertion>,
    q: Id<AssertionQualification>,
    site: Id<Occurrence>,
    work: &mut Work,
) -> Result<Option<NativeSupport>, ModelError> {
    native_for(
        native_context,
        supports,
        assertion,
        q,
        site,
        work,
        NativeQuestion::DeclaredClassInspection,
    )
}
/// Source-model ancestry inspection retains its report projection. The caller must
/// replay the complete MRO sequence; this grants no runtime class or read authority.
pub(super) fn source_class_hierarchy(
    native_context: NativeContext<'_>,
    supports: &Rows<symbols::ClassAncestrySupport>,
    assertion: Id<symbols::ClassAncestryObservation>,
    q: Id<AssertionQualification>,
    site: Id<Occurrence>,
    work: &mut Work,
) -> Result<Option<NativeSupport>, ModelError> {
    native_for(
        native_context,
        supports,
        assertion,
        q,
        site,
        work,
        NativeQuestion::SourceClassHierarchy,
    )
}
fn native_for<S: Support>(
    native_context: NativeContext<'_>,
    supports: &Rows<S>,
    assertion: Id<S::Assertion>,
    q: Id<AssertionQualification>,
    site: Id<Occurrence>,
    work: &mut Work,
    question: NativeQuestion,
) -> Result<Option<NativeSupport>, ModelError> {
    let NativeContext {
        data,
        entry,
        invocation,
    } = native_context;
    let mut selected = None;
    work.scan(supports.len())?;
    for support in supports.iter().filter(|s| s.assertion() == assertion) {
        work.tick()?;
        let Some(a) = support.attribution() else {
            continue;
        };
        let Some(run) = entry.runs.get(a.run) else {
            continue;
        };
        if run.input != invocation.input || run.context != invocation.context {
            continue;
        }
        let Some(qualification) = entry.qualifications.get(q) else {
            continue;
        };
        let Some(occurrence) = entry.occurrences.get(site) else {
            continue;
        };
        if qualification.context != invocation.context
            || (question == NativeQuestion::SourceClassHierarchy
                && (qualification.modality != Modality::Definite
                    || qualification.approximation != Approximation::Exact
                    || qualification.condition != conditions::Diagram::always().id()))
            || !input_scope(
                entry,
                qualification.scope,
                invocation.input,
                occurrence.source,
            )
        {
            continue;
        }
        let Some(n) = work
            .native
            .get(&(
                derivation::RowRef::of(assertion),
                derivation::RowRef::of(support.id()),
            ))
            .and_then(|id| data.native.get(*id))
            .filter(|n| n.qualification == q)
        else {
            continue;
        };
        let Some(pair) = data.premises.get(n.premise) else {
            continue;
        };
        if a.fidelity != n.fidelity || !native_authority(&a, pair, n, question) {
            continue;
        }
        if selected.is_some() {
            return Err(ModelError::Invalid(
                "ambiguous read provider support".into(),
            ));
        }
        selected = Some((n.premise, a.run, n.status));
    }
    Ok(selected)
}
/// The whole Base inventory checker reruns this operation over confirmed upstream rows. Roots
/// are selected by admission, never inferred from missing reads or chosen by the producer.
pub fn produce(data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,roots:&std::collections::BTreeSet<Id<SourceArtifact>>,budget:&resources::ResourceBudget)->Result<ReadRecords,ModelError> {produce_whole(data,entry,invocation,roots,budget,None)}
pub fn produce_with_local(data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,roots:&std::collections::BTreeSet<Id<SourceArtifact>>,budget:&resources::ResourceBudget,actual:ReadEntries<'_>)->Result<ReadRecords,ModelError> {produce_whole(data,entry,invocation,roots,budget,Some(actual))}
fn produce_whole(data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,roots:&std::collections::BTreeSet<Id<SourceArtifact>>,budget:&resources::ResourceBudget,actual:Option<ReadEntries<'_>>)->Result<ReadRecords,ModelError> {
    let mut out=ReadRecords::new(budget);
    let mut work=Work::new(data,entry,invocation,budget)?;
    produce_uses(data,entry,invocation,roots,&mut out,&mut work,None)?;
    produce_attributes(data,entry,invocation,roots,&mut out,&mut work,None)?;
    super::read_dynamic::produce_selected(data,entry,invocation,roots,&mut out,budget,&mut work,None,actual)?;
    let dynamic_scope=out.dynamic.iter().any(|row|matches!(row.kind,super::read_dynamic::DynamicKind::Exec|super::read_dynamic::DynamicKind::Eval));
    produce_formals(data,entry,invocation,roots,&mut out,budget,&mut work,None,dynamic_scope)?;
    super::read_fields::produce(data,entry,invocation,roots,&mut out,budget,&mut work,actual)?;
    Ok(out)
}

fn produce_uses(data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,roots:&std::collections::BTreeSet<Id<SourceArtifact>>,out:&mut ReadRecords,work:&mut Work,selected_id:Option<Id<FlowUseObservation>>)->Result<(),ModelError> {
    for observation in entry.use_observations.iter().filter(|row| selected_id.is_none_or(|id|row.id()==id)) {
        work.tick()?;
        if entry
            .qualifications
            .get(observation.qualification)
            .is_none_or(|q| q.context != invocation.context)
        {
            continue;
        }
        let Some(use_) = entry.uses.get(observation.use_) else {
            return Err(ModelError::Invalid("native read use missing".into()));
        };
        if !selected(entry, invocation, use_.occurrence, roots) {
            continue;
        }
        let Some(owner) = work.owner(use_.occurrence) else {
            continue;
        };
        let supported = native(
            super::read_channels::NativeContext {
                data,
                entry,
                invocation,
            },
            &entry.use_supports,
            observation.id(),
            observation.qualification,
            use_.occurrence,
            work,
        )?;
        let region = supported.and_then(|(_, run, _)| {
            conditions::entry::region_for_access(
                entry,
                owner,
                use_.occurrence,
                invocation.context,
                run,
            )
            .ok()
        });
        let region_premise = if let Some((region, support)) = region {
            let row = entry
                .regions
                .get(region)
                .ok_or_else(|| ModelError::Invalid("read region missing".into()))?;
            let pair = NativeAssertionPremise::Region {
                assertion: region,
                support,
            };
            work.qualification(data, pair.id())
                .filter(|n| n.qualification == row.qualification)
                .map(|n| n.premise)
        } else {
            None
        };
        let mut global = None;
        let mut global_binding = None;
        let mut global_ambiguous = false;
        for resolution in data.lexical_resolutions.iter().filter(|r| {
            r.read == use_.occurrence
                && entry
                    .qualifications
                    .get(r.qualification)
                    .is_some_and(|q| q.context == invocation.context)
        }) {
            work.tick()?;
            let Some(lexical::LexicalTarget::Binding { event }) =
                data.lexical_targets.get(resolution.target)
            else {
                continue;
            };
            if let Some(premise) = native(
                NativeContext {
                    data,
                    entry,
                    invocation,
                },
                &data.lexical_resolution_supports,
                resolution.id(),
                resolution.qualification,
                use_.occurrence,
                work,
            )? && let Some((binding, proof)) =
                native_binding(data, entry, invocation, *event, work)?
                && binding
                    .scope
                    .and_then(|s| entry.lexical_scopes.get(s))
                    .is_some_and(|s| s.kind == lexical::LexicalScopeKind::Module)
            {
                if global.is_some() {
                    global_ambiguous = true;
                }
                global = Some(premise.0);
                global_binding = Some(proof.0);
            }
        }
        let read = ReadObservation {
            invocation: invocation.id(),
            observation: observation.id(),
            owner,
            place: use_.place,
            location: if global_ambiguous {
                ReadLocation::UnresolvedGlobal
            } else if global.is_some() {
                ReadLocation::DeclaredGlobal
            } else {
                ReadLocation::NativePlace
            },
            lexical_premise: global.filter(|_| !global_ambiguous),
            binding_premise: global_binding.filter(|_| !global_ambiguous),
            qualification: observation.qualification,
            premise: supported.map(|p| p.0),
            status: supported.map_or(EvidenceStatus::Unresolved, |p| p.2),
            execution_region: region.filter(|_| region_premise.is_some()).map(|r| r.0),
            region_premise,
            reason: if supported.is_none() || (region_premise.is_none() && !observation.annotation)
            {
                Some(obligation::ObligationKind::MissingEvidence)
            } else {
                None
            },
        };
        let id = out.reads.insert(read)?;
        for value in entry.values.iter().filter(|v| {
            v.use_ == use_.id()
                && entry
                    .qualifications
                    .get(v.qualification)
                    .is_some_and(|q| q.context == invocation.context)
        }) {
            work.tick()?;
            let supported = native(
                super::read_channels::NativeContext {
                    data,
                    entry,
                    invocation,
                },
                &entry.value_supports,
                value.id(),
                value.qualification,
                use_.occurrence,
                work,
            )?;
            out.dependencies.insert(ReadDependency {
                read: id,
                observation: value.id(),
                premise: supported.map(|p| p.0),
                qualification: value.qualification,
                status: supported.map_or(EvidenceStatus::Unresolved, |p| p.2),
            })?;
        }
    }
    Ok(())
}

fn produce_attributes(data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,roots:&std::collections::BTreeSet<Id<SourceArtifact>>,out:&mut ReadRecords,work:&mut Work,selected_id:Option<Id<flow::FlowAttributeLoadObservation>>)->Result<(),ModelError> {
    for row in data.attribute_loads.iter().filter(|row| selected_id.is_none_or(|id|row.id()==id)) {
        work.tick()?;
        if entry
            .qualifications
            .get(row.qualification)
            .is_none_or(|q| q.context != invocation.context)
            || !selected(entry, invocation, row.occurrence, roots)
        {
            continue;
        }
        let Some(owner) = work.owner(row.occurrence) else {
            continue;
        };
        let supported = native(
            super::read_channels::NativeContext {
                data,
                entry,
                invocation,
            },
            &data.attribute_load_supports,
            row.id(),
            row.qualification,
            row.occurrence,
            work,
        )?;
        out.attributes.insert(AttributeRead {
            invocation: invocation.id(),
            observation: row.id(),
            owner,
            premise: supported.map(|p| p.0),
            qualification: row.qualification,
            status: supported.map_or(EvidenceStatus::Unresolved, |p| p.2),
        })?;
    }
    Ok(())
}

fn produce_formals(data:&EvaluationData,entry:&EntryData,invocation:&publication::AnalysisInvocation,roots:&std::collections::BTreeSet<Id<SourceArtifact>>,out:&mut ReadRecords,budget:&resources::ResourceBudget,work:&mut Work,selected_id:Option<Id<ParameterEntity>>,dynamic_scope:bool)->Result<(),ModelError> {
    // The admitted source parameter universe is independent of both emitted reads and assessments.
    for formal in entry.formals.iter().filter(|row| selected_id.is_none_or(|id|row.id()==id)) {
        work.tick()?;
        let ParameterEntity::Source { declaration } = formal else {
            continue;
        };
        if !selected(entry, invocation, *declaration, roots) {
            continue;
        }
        let Some(owner) = formal_owner(entry, *declaration) else {
            continue;
        };
        let EntityRef::Callable { callable } = entry
            .refs
            .get(owner)
            .ok_or_else(|| ModelError::Invalid("formal owner missing".into()))?
        else {
            continue;
        };
        let CallableEntity::Source {
            declaration: function,
            ..
        } = entry
            .callables
            .get(*callable)
            .ok_or_else(|| ModelError::Invalid("formal callable missing".into()))?
        else {
            continue;
        };
        let mut scopes = entry.lexical_scopes.iter().filter(|s| s.owner == *function);
        let scope = scopes
            .next()
            .filter(|_| scopes.next().is_none())
            .map(Record::id);
        let artifact = entry.occurrences.get(*declaration).unwrap().source;
        let mut coverages = entry.coverage.iter().filter(|c| {
            c.family == FactFamily::Flow
                && c.context == invocation.context
                && input_scope(entry, c.scope, invocation.input, artifact)
        });
        let coverage = coverages.next().filter(|_| coverages.next().is_none());
        let mut digest = KeySink::new("formal-native-read-universe");
        // The normalized callable owner decides which source body may support a negative.
        // Native reads stay observations even when an override or unknown wrapper owns behavior.
        work.scan(data.callable_assessments.len())?;
        let mut body = None;
        let mut ambiguous_body = false;
        for assessment in data
            .callable_assessments
            .iter()
            .filter(|a| a.callable == *callable && a.context == invocation.context)
        {
            assessment.id().encode(&mut digest);
            if body.replace(assessment).is_some() {
                ambiguous_body = true;
            }
        }
        let body_refusal = if ambiguous_body {
            Some(obligation::ObligationKind::ComparableConflict)
        } else {
            body.filter(|a| !a.body_admitted)
                .map(|a| {
                    use crate::domain::normalized::callables::CallableReason;
                    match a.body_reason {
                        CallableReason::BodyExcluded => obligation::ObligationKind::AbstractBody,
                        CallableReason::ConflictingEvidence => {
                            obligation::ObligationKind::ComparableConflict
                        }
                        CallableReason::MissingBodyEvidence
                        | CallableReason::MissingTraits
                        | CallableReason::IncompleteSyntax
                        | CallableReason::MissingSignature
                        | CallableReason::IncompleteSignature => {
                            obligation::ObligationKind::MissingEvidence
                        }
                        _ => obligation::ObligationKind::ScopeBoundary,
                    }
                })
                .or_else(|| {
                    body.is_none()
                        .then_some(obligation::ObligationKind::MissingEvidence)
                })
        };
        let mut seen = false;
        let mut unresolved = dynamic_scope;
        let reads = work.formal_reads.get(declaration);
        work.scan(reads.map_or(0, Vec::len))?;
        if let Some(reads) = work.formal_reads.get(declaration) {
            let mut ids = charged::ChargedSet::default();
            let mut charge = charged::StateCharge::new(budget, "formal-read-membership");
            for use_ in reads {
                ids.insert(&mut charge, *use_)?;
            }
            for use_ in ids.iter() {
                use_.encode(&mut digest);
                seen = true;
            }
        }
        // Nested/lazy bindings and arbitrary calls may inspect bindings beyond explicit native uses.
        if let Some(rows) = work.nested.get(&owner) {
            for row in rows {
                row.encode(&mut digest);
                unresolved = true;
            }
        }
        if let Some(rows) = work.calls.get(&owner) {
            for row in rows {
                row.encode(&mut digest);
                unresolved = true;
            }
        }
        let coverage_complete = coverage.is_some_and(|c| {
            c.status == CoverageStatus::CompleteUnderStatedModel
                && c.run.is_some_and(|run| {
                    entry.runs.get(run).is_some_and(|r| {
                        r.input == invocation.input && r.context == invocation.context
                    })
                })
        });
        for c in entry.coverage.iter().filter(|c| {
            c.family == FactFamily::Calls
                && c.context == invocation.context
                && input_scope(entry, c.scope, invocation.input, artifact)
        }) {
            c.id().encode(&mut digest);
        }
        if let Some(c) = coverage {
            c.id().encode(&mut digest);
        }
        if let Some(s) = scope {
            s.encode(&mut digest);
        }
        let (status, reason) = if seen {
            (ReadAssessment::ObservedRead, None)
        } else if unresolved {
            (
                ReadAssessment::Unknown,
                Some(obligation::ObligationKind::DynamicAccess),
            )
        } else if !coverage_complete
            || !calls_available(entry, invocation, artifact)
            || scope.is_none()
        {
            (
                ReadAssessment::Unknown,
                Some(obligation::ObligationKind::IncompleteCoverage),
            )
        } else if let Some(reason) = body_refusal {
            (ReadAssessment::Unknown, Some(reason))
        } else {
            (ReadAssessment::CompleteNoReadUnderModel, None)
        };
        out.formals.insert(FormalReadAssessment {
            invocation: invocation.id(),
            formal: formal.id(),
            owner,
            scope,
            coverage: coverage.map(Record::id),
            status,
            reason,
            reads: digest.finish(),
        })?;
    }
    Ok(())
}

// Parameter syntax is evaluated in its enclosing header owner. Its binding's callable owner
// is the unique nearest source callable anchor, independently of default evaluation ownership.
fn formal_owner(entry: &EntryData, parameter: Id<Occurrence>) -> Option<Id<EntityRef>> {
    let parameter = entry.occurrences.get(parameter)?;
    let mut best = None;
    let mut depth = 0;
    for callable in entry.callables.iter() {
        let CallableEntity::Source { declaration, .. } = callable else {
            continue;
        };
        let Some(function) = entry.occurrences.get(*declaration) else {
            continue;
        };
        if function.source != parameter.source
            || !parameter
                .structural_path
                .starts_with(&function.structural_path)
            || function.structural_path.len() >= parameter.structural_path.len()
        {
            continue;
        }
        if function.structural_path.len() > depth {
            depth = function.structural_path.len();
            best = Some(callable.id());
        } else if function.structural_path.len() == depth {
            return None;
        }
    }
    let reference = EntityRef::Callable { callable: best? };
    (entry.refs.get(reference.id()) == Some(&reference)).then_some(reference.id())
}

#[cfg(test)]
mod native_authority_tests {
    use super::*;
    fn id<R>(n: u8) -> Id<R> {
        serde_json::from_value(serde_json::json!(vec![n; 16])).unwrap()
    }
    fn frame(
        budget: &resources::ResourceBudget,
    ) -> (
        EvaluationData,
        EntryData,
        publication::AnalysisInvocation,
        AssertionQualification,
        Occurrence,
        ProviderRun,
    ) {
        let data = EvaluationData::new(budget);
        let mut entry = EntryData::new(budget);
        let input = id(1);
        let context = id(2);
        let scope = CoverageScope::Input { input };
        entry.scopes.insert(scope.clone()).unwrap();
        let q = AssertionQualification {
            context,
            scope: scope.id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            assumptions: assumptions::AssumptionSet::empty_id(),
        };
        let site = Occurrence {
            source: id(3),
            start: 0,
            end: 1,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Read,
            structural_path: vec![],
        };
        let run = ProviderRun {
            provider: id(4),
            context,
            input,
            configuration: ContentHash::of(b"configured"),
            requested_families: ContentHash::of(b"question"),
        };
        entry.qualifications.insert(q.clone()).unwrap();
        entry.occurrences.insert(site.clone()).unwrap();
        entry.runs.insert(run.clone()).unwrap();
        let (invocation, _) = publication::AnalysisInvocation::new(input, context, id(5), None, []);
        (data, entry, invocation, q, site, run)
    }
    #[test]
    fn native_read_keeps_native_citation_without_promoting_recognizer_or_mismatched_fidelity() {
        for scenario in 0..3 {
            let budget = resources::ResourceBudget::fixed(1 << 24).unwrap();
            let (mut data, entry, invocation, q, site, run) = frame(&budget);
            let native = lexical::LexicalResolutionSupport {
                assertion: id(6),
                run: run.id(),
                surface: id(7),
                evidence: id(8),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::NativeStructural,
            };
            let recognizer = lexical::LexicalResolutionSupport {
                origin: Origin::DerivedAnalysis,
                mode: ExtractionMode::Recognizer,
                fidelity: Fidelity::NormalizedStructural,
                ..native.clone()
            };
            let expected = NativeAssertionPremise::LexicalResolution {
                assertion: native.assertion,
                support: native.id(),
            };
            for support in
                std::iter::once(recognizer).chain((scenario != 1).then_some(native.clone()))
            {
                let pair = NativeAssertionPremise::LexicalResolution {
                    assertion: support.assertion,
                    support: support.id(),
                };
                let fidelity = if scenario == 2 && support.id() == native.id() {
                    Fidelity::NormalizedStructural
                } else {
                    support.fidelity
                };
                let n = analysis::native::NativeQualification {
                    premise: pair.id(),
                    qualification: q.id(),
                    family: FactFamily::Lexical,
                    fidelity,
                    status: EvidenceStatus::StructurallyObserved,
                };
                data.lexical_resolution_supports.insert(support).unwrap();
                data.premises.insert(pair).unwrap();
                data.native.insert(n).unwrap();
            }
            let mut work = Work::new(&data, &entry, &invocation, &budget).unwrap();
            let selected = super::native(
                NativeContext {
                    data: &data,
                    entry: &entry,
                    invocation: &invocation,
                },
                &data.lexical_resolution_supports,
                native.assertion,
                q.id(),
                site.id(),
                &mut work,
            )
            .unwrap();
            assert_eq!(
                selected,
                if scenario == 0 {
                    Some((
                        expected.id(),
                        run.id(),
                        EvidenceStatus::StructurallyObserved,
                    ))
                } else {
                    None
                }
            );
        }
    }
    #[test]
    fn native_read_preserves_raw_and_native_structural_canonical_source_syntax() {
        for fidelity in [Fidelity::Raw, Fidelity::NativeStructural] {
            let budget = resources::ResourceBudget::fixed(1 << 24).unwrap();
            let (mut data, entry, invocation, q, site, run) = frame(&budget);
            let support = SyntaxSupport {
                assertion: id(6),
                run: run.id(),
                surface: id(7),
                evidence: id(8),
                origin: Origin::SourceObservation,
                mode: ExtractionMode::NativeTraversal,
                fidelity,
            };
            let pair = NativeAssertionPremise::SyntaxObservation {
                assertion: support.assertion,
                support: support.id(),
            };
            let n = analysis::native::NativeQualification {
                premise: pair.id(),
                qualification: q.id(),
                family: FactFamily::Syntax,
                fidelity,
                status: EvidenceStatus::StructurallyObserved,
            };
            data.spelling_supports.insert(support.clone()).unwrap();
            data.premises.insert(pair.clone()).unwrap();
            data.native.insert(n).unwrap();
            let mut work = Work::new(&data, &entry, &invocation, &budget).unwrap();
            assert_eq!(
                super::native(
                    NativeContext {
                        data: &data,
                        entry: &entry,
                        invocation: &invocation
                    },
                    &data.spelling_supports,
                    support.assertion,
                    q.id(),
                    site.id(),
                    &mut work
                )
                .unwrap(),
                Some((pair.id(), run.id(), EvidenceStatus::StructurallyObserved))
            );
        }
    }
    #[test]
    fn class_field_source_support_is_native_syntax_not_an_analyzer_or_recognizer() {
        let pair = NativeAssertionPremise::ClassFieldSyntaxObservation {
            assertion: id(4),
            support: id(5),
        };
        for fidelity in [Fidelity::Raw, Fidelity::NativeStructural] {
            let n = analysis::native::NativeQualification {
                premise: pair.id(),
                qualification: id(6),
                family: FactFamily::Syntax,
                fidelity,
                status: EvidenceStatus::StructurallyObserved,
            };
            for origin in [
                Origin::SourceObservation,
                Origin::AnalyzerAssertion,
                Origin::DerivedAnalysis,
            ] {
                for mode in [ExtractionMode::NativeTraversal, ExtractionMode::Recognizer] {
                    let a = SupportAttribution {
                        run: id(1),
                        surface: id(2),
                        evidence: id(3),
                        origin,
                        mode,
                        fidelity,
                    };
                    assert_eq!(
                        native_authority(&a, &pair, &n, NativeQuestion::ExecutableRead),
                        origin == Origin::SourceObservation
                            && mode == ExtractionMode::NativeTraversal
                    );
                }
            }
        }
    }
    #[test]
    fn report_projection_is_only_a_named_declared_class_inspection_premise() {
        let a = SupportAttribution {
            run: id(1),
            surface: id(2),
            evidence: id(3),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::ReportProjection,
        };
        for pair in [
            NativeAssertionPremise::FunctionTraitObservation {
                assertion: id(4),
                support: id(5),
            },
            NativeAssertionPremise::SymbolDeclaration {
                assertion: id(4),
                support: id(5),
            },
            NativeAssertionPremise::ParameterDeclaration {
                assertion: id(4),
                support: id(5),
            },
        ] {
            let n = analysis::native::NativeQualification {
                premise: pair.id(),
                qualification: id(6),
                family: FactFamily::Signatures,
                fidelity: Fidelity::ReportProjection,
                status: EvidenceStatus::StructurallyObserved,
            };
            assert!(native_authority(
                &a,
                &pair,
                &n,
                NativeQuestion::DeclaredClassInspection
            ));
            assert!(!native_authority(
                &a,
                &pair,
                &n,
                NativeQuestion::ExecutableRead
            ));
            assert!(!native_authority(
                &a,
                &pair,
                &analysis::native::NativeQualification {
                    fidelity: Fidelity::NativeStructural,
                    ..n
                },
                NativeQuestion::DeclaredClassInspection
            ));
        }
        let pair = NativeAssertionPremise::Use {
            assertion: id(4),
            support: id(5),
        };
        let n = analysis::native::NativeQualification {
            premise: pair.id(),
            qualification: id(6),
            family: FactFamily::Flow,
            fidelity: Fidelity::ReportProjection,
            status: EvidenceStatus::StructurallyObserved,
        };
        assert!(!native_authority(
            &a,
            &pair,
            &n,
            NativeQuestion::DeclaredClassInspection
        ));
    }
    #[test]
    fn projected_mro_has_only_bounded_source_hierarchy_authority() {
        let a = SupportAttribution {
            run: id(1),
            surface: id(2),
            evidence: id(3),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::ReportProjection,
        };
        let pair = NativeAssertionPremise::ClassAncestryObservation {
            assertion: id(4),
            support: id(5),
        };
        let n = analysis::native::NativeQualification {
            premise: pair.id(),
            qualification: id(6),
            family: FactFamily::Signatures,
            fidelity: Fidelity::ReportProjection,
            status: EvidenceStatus::StructurallyObserved,
        };
        assert!(native_authority(
            &a,
            &pair,
            &n,
            NativeQuestion::SourceClassHierarchy
        ));
        assert!(!native_authority(
            &a,
            &pair,
            &n,
            NativeQuestion::ExecutableRead
        ));
        assert!(!native_authority(
            &a,
            &pair,
            &n,
            NativeQuestion::DeclaredClassInspection
        ));
        for changed in [
            SupportAttribution {
                origin: Origin::DerivedAnalysis,
                ..a
            },
            SupportAttribution {
                mode: ExtractionMode::Recognizer,
                ..a
            },
            SupportAttribution {
                fidelity: Fidelity::NativeStructural,
                ..a
            },
        ] {
            assert!(!native_authority(
                &changed,
                &pair,
                &n,
                NativeQuestion::SourceClassHierarchy
            ));
        }
        let use_ = NativeAssertionPremise::Use {
            assertion: id(4),
            support: id(5),
        };
        assert!(!native_authority(
            &a,
            &use_,
            &n,
            NativeQuestion::SourceClassHierarchy
        ));
        let structural = SupportAttribution {
            fidelity: Fidelity::NativeStructural,
            ..a
        };
        let structural_n = analysis::native::NativeQualification {
            fidelity: Fidelity::NativeStructural,
            ..n
        };
        assert!(!native_authority(
            &structural,
            &use_,
            &structural_n,
            NativeQuestion::SourceClassHierarchy
        ));
    }
    #[test]
    fn declared_inspection_run_selection_does_not_promote_a_recognizer_duplicate() {
        for recognizer_only in [false, true] {
            let budget = resources::ResourceBudget::fixed(1 << 24).unwrap();
            let (mut data, entry, invocation, q, site, run) = frame(&budget);
            let report = symbols::FunctionTraitSupport {
                assertion: id(6),
                run: run.id(),
                surface: id(7),
                evidence: id(8),
                origin: Origin::AnalyzerAssertion,
                mode: ExtractionMode::NativeTraversal,
                fidelity: Fidelity::ReportProjection,
            };
            let recognizer = symbols::FunctionTraitSupport {
                origin: Origin::DerivedAnalysis,
                mode: ExtractionMode::Recognizer,
                fidelity: Fidelity::NormalizedStructural,
                ..report.clone()
            };
            let expected = NativeAssertionPremise::FunctionTraitObservation {
                assertion: report.assertion,
                support: report.id(),
            };
            for support in
                std::iter::once(recognizer).chain((!recognizer_only).then_some(report.clone()))
            {
                let pair = NativeAssertionPremise::FunctionTraitObservation {
                    assertion: support.assertion,
                    support: support.id(),
                };
                let n = analysis::native::NativeQualification {
                    premise: pair.id(),
                    qualification: q.id(),
                    family: FactFamily::Signatures,
                    fidelity: support.fidelity,
                    status: EvidenceStatus::StructurallyObserved,
                };
                data.function_trait_supports.insert(support).unwrap();
                data.premises.insert(pair).unwrap();
                data.native.insert(n).unwrap();
            }
            let mut work = Work::new(&data, &entry, &invocation, &budget).unwrap();
            assert_eq!(
                super::native(
                    NativeContext {
                        data: &data,
                        entry: &entry,
                        invocation: &invocation
                    },
                    &data.function_trait_supports,
                    report.assertion,
                    q.id(),
                    site.id(),
                    &mut work
                )
                .unwrap(),
                None
            );
            assert_eq!(
                super::declared_class_inspection(
                    NativeContext {
                        data: &data,
                        entry: &entry,
                        invocation: &invocation
                    },
                    &data.function_trait_supports,
                    report.assertion,
                    q.id(),
                    site.id(),
                    &mut work
                )
                .unwrap(),
                if recognizer_only {
                    None
                } else {
                    Some((
                        expected.id(),
                        run.id(),
                        EvidenceStatus::StructurallyObserved,
                    ))
                }
            );
        }
    }
}

#[cfg(test)]
mod scoped_read_controls {
    use super::*;
    use lexical::*;
    use syntax::*;
    fn id<R>(n:u8)->Id<R>{serde_json::from_value(serde_json::json!(vec![n;16])).unwrap()}
    fn fixture(budget:&resources::ResourceBudget,scenario:u8)->(EvaluationData,EntryData,publication::AnalysisInvocation,std::collections::BTreeSet<Id<SourceArtifact>>) {
        let mut data=EvaluationData::new(budget);let mut entry=EntryData::new(budget);
        let artifact=SourceArtifact::from_bytes(id(1),"reads.py".into(),b"def f(p): pass\nclass C: x=1\nexec(code)\n").unwrap();
        let scope=CoverageScope::Artifact {artifact:artifact.id()};let context=id(2);
        let q=AssertionQualification {context,scope:scope.id(),condition:conditions::Diagram::always().id(),modality:Modality::Definite,approximation:Approximation::Exact,assumptions:assumptions::AssumptionSet::empty_id()};
        let run=ProviderRun {provider:id(4),context,input:artifact.input,configuration:ContentHash::of(b"configured"),requested_families:ContentHash::of(b"reads")};
        let invocation=publication::AnalysisInvocation::new(artifact.input,context,id(5),None,[]).0;
        data.artifacts.insert(artifact.clone()).unwrap();entry.artifacts.insert(artifact.clone()).unwrap();
        data.uses.insert(input::ArtifactUse {artifact:artifact.id(),input:artifact.input,role:input::SourceRole::Release}).unwrap();
        entry.scopes.insert(scope.clone()).unwrap();entry.qualifications.insert(q.clone()).unwrap();data.qualifications.insert(q.clone()).unwrap();entry.runs.insert(run.clone()).unwrap();
        for family in [FactFamily::Flow,FactFamily::Calls] {entry.coverage.insert(ProviderCoverage {scope:scope.id(),provider:Some(run.provider),context,family,run:Some(run.id()),status:CoverageStatus::CompleteUnderStatedModel,reason:None,diagnostic:None}).unwrap();}
        let function=Occurrence {source:artifact.id(),start:0,end:14,syntax_kind:SyntaxKind::StmtFunctionDef,role:OccurrenceRole::Declaration,structural_path:vec![0]};
        let parameter=Occurrence {start:6,end:7,syntax_kind:SyntaxKind::Parameter,role:OccurrenceRole::Declaration,structural_path:vec![0,0],..function.clone()};
        let callable=CallableEntity::Source {declaration:function.id(),kind:CallableKind::Function};let owner=EntityRef::Callable {callable:callable.id()};
        let formal=ParameterEntity::Source {declaration:parameter.id()};
        entry.callables.insert(callable.clone()).unwrap();entry.refs.insert(owner.clone()).unwrap();entry.formals.insert(formal).unwrap();
        entry.lexical_scopes.insert(LexicalScope {owner:function.id(),kind:LexicalScopeKind::Function}).unwrap();
        use normalized::callables::*;
        data.callable_assessments.insert(EffectiveCallableAssessment {callable:callable.id(),context,decorators:ContentHash::of(b"empty"),policy:ContentHash::of(b"policy"),identity:Knowledge::Known,identity_reason:CallableReason::EvidenceAgreement,signatures:Knowledge::Known,signature_reason:CallableReason::EvidenceAgreement,descriptor:Knowledge::Known,descriptor_kind:Some(DescriptorKind::Function),descriptor_reason:CallableReason::EvidenceAgreement,body:Knowledge::Known,body_admitted:true,body_reason:CallableReason::EvidenceAgreement,asynchronous:Some(false),generator:Some(false)}).unwrap();
        let class=Occurrence {start:15,end:27,syntax_kind:SyntaxKind::StmtClassDef,structural_path:vec![1],..function.clone()};
        let site=Occurrence {start:24,end:25,syntax_kind:SyntaxKind::ExprName,role:OccurrenceRole::Binding,structural_path:vec![1,0],..class.clone()};
        let value=Occurrence {start:26,end:27,syntax_kind:SyntaxKind::ExprNumberLiteral,role:OccurrenceRole::Syntax,structural_path:vec![1,1],..class.clone()};
        let class_entity=ClassEntity::Source {declaration:class.id()};data.classes.insert(class_entity.clone()).unwrap();
        let field=ClassFieldSyntaxObservation {qualification:q.id(),class:class.id(),target:site.id(),annotation:None,value:Some(value.id())};
        let support=ClassFieldSyntaxSupport {assertion:field.id(),run:run.id(),surface:id(6),evidence:id(7),origin:Origin::SourceObservation,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::Raw};
        let premise=NativeAssertionPremise::ClassFieldSyntaxObservation {assertion:field.id(),support:support.id()};
        data.class_fields.insert(field).unwrap();data.class_field_supports.insert(support).unwrap();data.native.insert(analysis::native::NativeQualification {premise:premise.id(),qualification:q.id(),family:FactFamily::Syntax,fidelity:Fidelity::Raw,status:EvidenceStatus::StructurallyObserved}).unwrap();data.premises.insert(premise).unwrap();
        data.binding_events.insert(BindingEvent {site:site.id(),name:"x".into()}).unwrap();
        let mut occurrences=vec![function,parameter,class,site,value];
        if scenario==1 {
            let access=Occurrence {start:9,end:10,syntax_kind:SyntaxKind::ExprAttribute,role:OccurrenceRole::Read,structural_path:vec![0,1],..occurrences[0].clone()};
            let own=OccurrenceOwnership {occurrence:access.id(),owner:occurrences[0].id(),entity:owner.id()};entry.owners.insert(own).unwrap();
            let load=flow::FlowAttributeLoadObservation {qualification:q.id(),occurrence:access.id(),name:"x".into()};
            let support=flow::FlowAttributeLoadSupport {assertion:load.id(),run:run.id(),surface:id(6),evidence:id(8),origin:Origin::AnalyzerAssertion,mode:ExtractionMode::NativeTraversal,fidelity:Fidelity::NativeStructural};
            let premise=NativeAssertionPremise::FlowAttributeLoadObservation {assertion:load.id(),support:support.id()};
            data.native.insert(analysis::native::NativeQualification {premise:premise.id(),qualification:q.id(),family:FactFamily::Flow,fidelity:Fidelity::NativeStructural,status:EvidenceStatus::StructurallyObserved}).unwrap();data.premises.insert(premise).unwrap();data.attribute_loads.insert(load).unwrap();data.attribute_load_supports.insert(support).unwrap();occurrences.push(access);
        }
        if scenario==2 {
            let call=Occurrence {start:28,end:38,syntax_kind:SyntaxKind::ExprCall,role:OccurrenceRole::Syntax,structural_path:vec![2],..occurrences[0].clone()};
            let callee=Occurrence {start:28,end:32,syntax_kind:SyntaxKind::ExprName,role:OccurrenceRole::Read,structural_path:vec![2,0],..call.clone()};
            entry.owners.insert(OccurrenceOwnership {occurrence:call.id(),owner:occurrences[0].id(),entity:owner.id()}).unwrap();
            data.references.insert(ReferenceObservation {qualification:q.id(),read:callee.id(),scope:id(9),parent:call.id(),field:SyntaxField::Callee,name:"exec".into()}).unwrap();
            data.call_syntax.insert(calls::CallSyntax {qualification:q.id(),site:call.id(),callee:callee.id(),arguments:ContentHash::of(b"args"),in_annotation:false}).unwrap();occurrences.extend([call,callee]);
        }
        for occurrence in occurrences {entry.occurrences.insert(occurrence.clone()).unwrap();data.occurrences.insert(occurrence).unwrap();}
        (data,entry,invocation,[artifact.id()].into_iter().collect())
    }
    #[test]
    fn complete_read_phases_match_finite_whole_oracle_and_preserve_global_uncertainty() {
        for scenario in 0..3 {
            let budget=resources::ResourceBudget::fixed(16<<20).unwrap();
            let (data,entry,invocation,roots)=fixture(&budget,scenario);
            let expected=produce(&data,&entry,&invocation,&roots,&budget).unwrap();
            let mut prepared=PreparedReads::new(&invocation,&budget);
            for n in data.native.iter(){let premise=data.premises.get(n.premise).unwrap();prepared.prepare(ReadMetadata::Native {pair:premise.assertion_and_support(),premise:n.premise,qualification:n.id()}).unwrap();}
            for own in entry.owners.iter(){prepared.prepare(ReadMetadata::Owner {occurrence:own.occurrence,entity:own.entity}).unwrap();}
            for occurrence in entry.occurrences.iter().filter(|o|o.syntax_kind==SyntaxKind::ExprCall){if let Some(owner)=prepared.work.owner(occurrence.id()){prepared.prepare(ReadMetadata::Call {owner,occurrence:occurrence.id()}).unwrap();}}
            let actual=local_semantics::ProducedLocal::empty(&budget);let witnesses=Rows::new(&budget);let sources=Rows::new(&budget);let values=ReadEntries {witnesses:&witnesses,sources:&sources,actual:&actual};
            let mut observed=ReadRecords::new(&budget);
            for row in entry.use_observations.iter(){observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::Use(row.id()),&budget,values).unwrap()).unwrap();}
            for row in data.attribute_loads.iter(){observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::Attribute(row.id()),&budget,values).unwrap()).unwrap();}
            for row in data.call_syntax.iter(){observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::DynamicCall(row.id()),&budget,values).unwrap()).unwrap();}
            for row in data.attribute_loads.iter(){observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::DynamicAttribute(row.id()),&budget,values).unwrap()).unwrap();}
            for row in entry.formals.iter(){observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::Formal(row.id()),&budget,values).unwrap()).unwrap();}
            prepared.field_roots(&data,&entry,&invocation,&roots).unwrap();prepared.begin_field_candidates().unwrap();
            for row in data.call_syntax.iter(){observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::FieldCall(row.id()),&budget,values).unwrap()).unwrap();}
            for row in data.class_fields.iter(){observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::FieldClass(row.id()),&budget,values).unwrap()).unwrap();}
            for row in entry.definition_observations.iter(){observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::FieldStore(row.id()),&budget,values).unwrap()).unwrap();}
            for row in data.bindings.iter(){observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::FieldGlobal(row.id()),&budget,values).unwrap()).unwrap();}
            prepared.seal_fields().unwrap();
            let keys=prepared.field_keys().cloned().collect::<Vec<_>>();
            for (class,name) in keys {observed.append(&prepared.produce(&data,&entry,&invocation,&roots,ReadRoot::FieldAssessment {class,name},&budget,values).unwrap()).unwrap();}
            assert!(observed.same(&expected));
            let field=observed.fields.assessments.iter().next().unwrap();let formal=observed.formals.iter().next().unwrap();
            assert_eq!(field.status,[ReadAssessment::CompleteNoReadUnderModel,ReadAssessment::ObservedRead,ReadAssessment::Unknown][scenario as usize]);
            assert_eq!(formal.status,if scenario==2 {ReadAssessment::Unknown}else{ReadAssessment::CompleteNoReadUnderModel});
            if scenario==2 {assert_eq!(field.reason,Some(obligation::ObligationKind::DynamicAccess));assert_eq!(formal.reason,field.reason);}
            drop(observed);drop(expected);drop(prepared);drop(actual);drop(witnesses);drop(sources);drop(data);drop(entry);assert_eq!(budget.reserved(),0);
        }
    }
    #[test]
    fn empty_prepared_reads_keep_one_work_bound_and_refuse_changed_frame() {
        let budget=resources::ResourceBudget::fixed(1<<20).unwrap();let invocation=publication::AnalysisInvocation::new(id(1),id(2),id(3),None,[]).0;
        let mut prepared=PreparedReads::new(&invocation,&budget);prepared.scan_metadata(WORK_LIMIT-1).unwrap();prepared.scan_metadata(1).unwrap();assert!(prepared.scan_metadata(1).is_err());
        let changed=publication::AnalysisInvocation {context:id(4),..invocation};assert!(prepared.frame(&changed).is_err());drop(prepared);assert_eq!(budget.reserved(),0);
    }
}
