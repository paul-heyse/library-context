//! Total projection selection over normalized records. Store eligibility validates upstream
//! invariants; this operation describes topology and never grants call/composition authority.
use super::*;
use super::compact::{Metadata,PrimitiveMetadata,CoverageFrame};
use crate::domain::{
    assertion::AssertionQualification,
    attribution::*,
    calls::*,
    charged::{ChargedMap, ChargedSet, StateCharge},
    normalized::Rows,
    resources::ResourceBudget,
    source::*,
};
macro_rules! inputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct ProjectionData { $(pub $field: Rows<$ty>,)* }
        impl ProjectionData {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if name == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { crate::domain::normalized::facts_inputs(vec![$(ValidationInput::of::<$ty>(&["id"]),)*]) }
            pub fn stage_inputs() -> Vec<stages::RelationUse> { vec![$(stages::RelationUse::completed::<$ty>()),*] }
        }
    }
}
crate::projection_inputs!(inputs);
macro_rules! outputs {
    ($($field:ident: $ty:ty,)*) => {
        pub struct ProjectionOutput { $(pub $field: Rows<$ty>,)* }
        impl ProjectionOutput {
            pub fn new(budget: &ResourceBudget) -> Self { Self { $($field: Rows::new(budget),)* } }
            pub fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
                $(if name == <$ty>::NAME { self.$field.decode(batch)?; return Ok(true); })* Ok(false)
            }
            pub fn validation_inputs() -> Vec<ValidationInput> { vec![$(ValidationInput::of::<$ty>(&["id"]),)*] }
        }
    }
}
crate::projection_outputs!(outputs);
pub(super) fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("projection requires {}", R::NAME)))
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct ProjectionKey {
    pub input: Id<InputRevision>,
    pub context: Id<AnalysisContext>,
    pub name: ProjectionName,
}
impl HeapSize for ProjectionKey {}
/// A complete selected universe, typed arcs and side records. Request selectors never affect it.
/// This is a structural projection, not an assertion that supplied records passed publication.
pub struct ProjectionInput {
    key: ProjectionKey,
    pub(super) vertices: Rows<EntityRef>,
    pub(super) arcs: ChargedMap<ArcId, Arc>,
    pub(super) subjects: Rows<ProjectionGapSubject>,
    pub(super) gaps: Rows<ProjectionGap>,
    pub(super) coverage: Rows<ProjectionSourceCoverage>,
    charge: StateCharge,
}
impl ProjectionInput {
    pub fn key(&self) -> ProjectionKey {
        self.key
    }
    pub fn vertices(&self) -> impl Iterator<Item = &EntityRef> {
        self.vertices.iter()
    }
    pub fn arcs(&self) -> impl Iterator<Item = &Arc> {
        self.arcs.values()
    }
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
    pub fn arc_count(&self) -> usize {
        self.arcs.len()
    }
    pub fn gaps(&self) -> impl Iterator<Item = &ProjectionGap> {
        self.gaps.iter()
    }
    pub fn coverage(&self) -> impl Iterator<Item = &ProjectionSourceCoverage> {
        self.coverage.iter()
    }
    pub fn subjects(&self) -> impl Iterator<Item = &ProjectionGapSubject> {
        self.subjects.iter()
    }
    pub fn assessment(&self) -> ProjectionSourceAssessment {
        ProjectionSourceAssessment {
            input: self.key.input,
            context: self.key.context,
            projection: self.key.name,
            version: ProjectionSpec::VERSION,
            vertices: self.vertices.len() as i64,
            arcs: self.arcs.len() as i64,
            gaps: self.gaps.len() as i64,
            availability: if self.gaps.iter().any(|g| {
                !matches!(
                    g.reason,
                    ProjectionGapReason::OutsidePolicy | ProjectionGapReason::Builtin
                )
            }) {
                ProjectionAvailability::Partial
            } else {
                ProjectionAvailability::CompleteUnderStatedModel
            },
        }
    }
    fn gap(
        &mut self,
        subject: ProjectionGapSubject,
        reason: ProjectionGapReason,
    ) -> Result<(), ModelError> {
        let subject = self.subjects.insert(subject)?;
        self.gaps.insert(ProjectionGap {
            assessment: self.assessment().id(),
            subject,
            reason,
        })?;
        Ok(())
    }
    fn arc(
        &mut self,
        id: ArcId,
        source: Id<EntityRef>,
        target: Id<EntityRef>,
    ) -> Result<(), ModelError> {
        if self.vertices.get(source).is_none() || self.vertices.get(target).is_none() {
            return Err(invalid(
                "projection arc endpoint outside its declared universe",
            ));
        }
        let arc = Arc { id, source, target };
        if let Some(previous) = self.arcs.insert(&mut self.charge, id, arc)?
            && previous != arc
        {
            return Err(invalid("conflicting projection arc"));
        }
        Ok(())
    }
}
struct Index<'a> {
    definitions: ChargedMap<Id<Occurrence>, Id<CallableEntity>>,
    assessments: ChargedMap<Id<NormalizedCallEvent>, &'a EventAssessment>,
    alternatives: ChargedMap<Id<NormalizedCallEvent>, Vec<&'a NormalizedCallAlternative>>,
    invocations: ChargedSet<Id<NormalizedCallAlternative>>,
    imports: ChargedMap<Id<ImportModuleAssessment>, Vec<&'a ImportModuleCandidate>>,
    references: ChargedMap<Id<ReferenceEntityAssessment>, Vec<&'a ReferenceEntityCandidate>>,
    exposures: ChargedMap<Id<PublicExposure>, Vec<&'a PublicExposureCandidate>>,
    _charge: StateCharge,
}
impl<'a> Index<'a> {
    fn new(data: &'a ProjectionData, budget: &ResourceBudget) -> Result<Self, ModelError> {
        let mut out = Self {
            definitions: Default::default(),
            assessments: Default::default(),
            alternatives: Default::default(),
            invocations: Default::default(),
            imports: Default::default(),
            references: Default::default(),
            exposures: Default::default(),
            _charge: StateCharge::new(budget, "projection-index"),
        };
        for callable in data.callables.iter() {
            if let CallableEntity::Source { declaration, .. } = callable
                && out
                    .definitions
                    .insert(&mut out._charge, *declaration, callable.id())?
                    .is_some()
            {
                return Err(invalid("duplicate source callable declaration"));
            }
        }
        for row in data.event_assessments.iter() {
            if out
                .assessments
                .insert(&mut out._charge, row.event, row)?
                .is_some()
            {
                return Err(invalid("duplicate projection event assessment"));
            }
        }
        for row in data.alternatives.iter() {
            out.alternatives
                .update(&mut out._charge, row.event, |v| v.push(row))?;
        }
        for row in data.admissions.iter() {
            let policy = need(&data.policies, row.assessment)?;
            let alternative = need(&data.alternatives, row.alternative)?;
            if alternative.event != policy.event {
                return Err(invalid("projection policy event mismatch"));
            }
            if policy.policy == CallPolicy::Invocation {
                out.invocations.insert(&mut out._charge, row.alternative)?;
            }
        }
        for row in data.import_candidates.iter() {
            out.imports
                .update(&mut out._charge, row.assessment, |v| v.push(row))?;
        }
        for row in data.reference_candidates.iter() {
            out.references
                .update(&mut out._charge, row.assessment, |v| v.push(row))?;
        }
        for row in data.exposure_candidates.iter() {
            out.exposures
                .update(&mut out._charge, row.exposure, |v| v.push(row))?;
        }
        Ok(out)
    }
}
impl Metadata for ProjectionData {
    fn coverages(&self)->Box<dyn Iterator<Item=CoverageFrame>+'_>{Box::new(self.coverage.iter().map(|row|CoverageFrame{id:row.id(),scope:row.scope,context:row.context,family:row.family,status:row.status}))}
    fn artifact_input(&self,id:Id<SourceArtifact>)->Result<Id<InputRevision>,ModelError>{Ok(need(&self.artifacts,id)?.input)}
    fn occurrence_source(&self,id:Id<Occurrence>)->Result<Id<SourceArtifact>,ModelError>{Ok(need(&self.occurrences,id)?.source)}
    fn module_source(&self,id:Id<Module>)->Result<Id<SourceArtifact>,ModelError>{Ok(need(&self.modules,id)?.source)}
    fn symbol_frame(&self,id:Id<ProviderSymbol>)->Result<(Id<AnalysisContext>,Id<ProviderModule>),ModelError>{let symbol=need(&self.symbols,id)?;Ok((symbol.context,symbol.module))}
    fn acquired_module(&self,id:Id<ProviderModule>)->Result<Option<Id<Module>>,ModelError>{Ok(match need(&self.provider_modules,id)?{ProviderModule::Acquired{module}=>Some(*module),_=>None})}
    fn field_class(&self,id:Id<FieldEntity>)->Result<Id<ClassEntity>,ModelError>{Ok(need(&self.fields,id)?.class)}
    fn import_frame(&self,id:Id<syntax::ImportAliasObservation>)->Result<(Id<AssertionQualification>,Id<Occurrence>),ModelError>{let row=need(&self.imports,id)?;Ok((row.qualification,row.alias))}
    fn reference_frame(&self,id:Id<lexical::ReferenceObservation>)->Result<(Id<AssertionQualification>,Id<Occurrence>),ModelError>{let row=need(&self.references,id)?;Ok((row.qualification,row.read))}
}
impl ProjectionData {
    fn scope_input(&self,metadata:&impl Metadata,scope:Id<CoverageScope>)->Option<Id<InputRevision>> {
        match self.scopes.get(scope)? {
            CoverageScope::Input{input}=>Some(*input),
            CoverageScope::Artifact{artifact}=>metadata.artifact_input(*artifact).ok(),
            CoverageScope::Module{module}=>metadata.module_source(*module).and_then(|source|metadata.artifact_input(source)).ok(),
            CoverageScope::Release{..}=>None,
        }
    }
    fn occurrence_input(&self, metadata:&impl Metadata, id: Id<Occurrence>) -> Result<Id<InputRevision>, ModelError> {
        metadata.artifact_input(metadata.occurrence_source(id)?)
    }
    fn symbol_in(&self,metadata:&impl Metadata,id:Id<ProviderSymbol>,key:ProjectionKey)->Result<bool,ModelError>{
        let (context,module)=metadata.symbol_frame(id)?;
        Ok(context==key.context && match metadata.acquired_module(module)? {
            Some(module)=>metadata.artifact_input(metadata.module_source(module)?)?==key.input,
            None=>true,
        })
    }
    fn entity_in(&self,metadata:&impl Metadata,entity:&EntityRef,key:ProjectionKey)->Result<bool,ModelError>{
        Ok(match entity {
            EntityRef::Module{module}=>metadata.artifact_input(metadata.module_source(*module)?)?==key.input,
            EntityRef::Occurrence{occurrence}=>self.occurrence_input(metadata,*occurrence)?==key.input,
            EntityRef::Callable{callable}=>match need(&self.callables,*callable)?{
                CallableEntity::Source{declaration,..}=>self.occurrence_input(metadata,*declaration)?==key.input,
                CallableEntity::Synthetic{symbol}|CallableEntity::External{symbol}=>self.symbol_in(metadata,*symbol,key)?,
            },
            EntityRef::Class{class}=>match need(&self.classes,*class)?{
                ClassEntity::Source{declaration}=>self.occurrence_input(metadata,*declaration)?==key.input,
                ClassEntity::Synthetic{symbol}|ClassEntity::External{symbol}=>self.symbol_in(metadata,*symbol,key)?,
            },
            EntityRef::Parameter{parameter}=>match need(&self.parameters,*parameter)?{
                ParameterEntity::Source{declaration}=>self.occurrence_input(metadata,*declaration)?==key.input,
                ParameterEntity::NativeSlot{callable,..}=>self.entity_in(metadata,&EntityRef::Callable{callable:*callable},key)?,
            },
            EntityRef::Field{field}=>self.entity_in(metadata,&EntityRef::Class{class:metadata.field_class(*field)?},key)?,
            EntityRef::Type{..}|EntityRef::Place{..}=>false,
        })
    }
    fn qualified(&self,metadata:&impl Metadata,q:Id<AssertionQualification>,key:ProjectionKey)->Result<bool,ModelError>{
        let q=need(&self.qualifications,q)?;
        Ok(q.context==key.context && self.scope_input(metadata,q.scope)==Some(key.input))
    }
    pub fn keys(&self,budget:&ResourceBudget)->Result<ProjectionKeys,ModelError>{self.keys_with(self,budget)}
    fn keys_with(&self,metadata:&impl Metadata,budget:&ResourceBudget)->Result<ProjectionKeys,ModelError>{
        let mut out=ProjectionKeys{keys:Default::default(),_charge:StateCharge::new(budget,"projection-keys")};
        for run in self.runs.iter(){for name in ProjectionName::ALL{out.keys.insert(&mut out._charge,ProjectionKey{input:run.input,context:run.context,name})?;}}
        for coverage in metadata.coverages(){if let Some(input)=self.scope_input(metadata,coverage.scope){for name in ProjectionName::ALL{out.keys.insert(&mut out._charge,ProjectionKey{input,context:coverage.context,name})?;}}}
        Ok(out)
    }
}
/// Production topology keeps primitive source correspondence, never paths, native spellings or
/// occurrence structural paths. The ordinary rich diagnostic input is a separate API.
pub struct CompactProjectionData { data:ProjectionData, metadata:PrimitiveMetadata }
impl CompactProjectionData {
    pub fn new(budget:&ResourceBudget)->Self{Self{data:ProjectionData::new(budget),metadata:PrimitiveMetadata::new(budget)}}
    pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError>{
        if self.metadata.visit(name,batch)?{return Ok(true);}
        self.data.visit(name,batch)
    }
    pub fn prepare(&self,budget:&ResourceBudget)->Result<PreparedProjections<'_>,ModelError>{
        Ok(PreparedProjections{data:&self.data,metadata:&self.metadata,index:Index::new(&self.data,budget)?,keys:self.data.keys_with(&self.metadata,budget)?})
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AdmissionScope {Canonical,Snapshot}
impl AdmissionScope {
    pub fn roots(self)->Vec<ValidationInput>{
        match self {
            Self::Canonical=>vec![ValidationInput::of::<ProviderRun>(&["id"]),ValidationInput::of::<ProviderCoverage>(&["id"]),ValidationInput::of::<ProjectionSourceAssessment>(&["id"]),ValidationInput::of::<ProjectionGap>(&["id"]),ValidationInput::of::<ProjectionGapSubject>(&["id"]),ValidationInput::of::<ProjectionSourceCoverage>(&["id"])],
            Self::Snapshot=>vec![ValidationInput::of::<ProjectionSnapshot>(&["id"]),ValidationInput::of::<ProjectionSnapshotChunk>(&["id"])],
        }
    }
}
pub struct PreparedProjections<'a>{data:&'a ProjectionData,metadata:&'a PrimitiveMetadata,index:Index<'a>,keys:ProjectionKeys}
impl PreparedProjections<'_>{
    pub fn keys(&self)->impl Iterator<Item=ProjectionKey>+'_ {self.keys.iter()}
    pub fn contains(&self,key:ProjectionKey)->bool{self.keys.keys.contains(&key)}
    pub fn validate_key(&self,stored:&ProjectionOutput,key:ProjectionKey,snapshots:bool,budget:&ResourceBudget)->Result<(),ModelError>{
        if !self.keys.keys.contains(&key){return Err(invalid("advertised projection has no actual input/context collection"));}
        let input=describe_indexed(self.data,&self.index,self.metadata,key,budget)?;
        let assessment=input.assessment();
        if stored.assessments.len()!=1 || stored.assessments.get(assessment.id())!=Some(&assessment)
            || !stored.subjects.same(&input.subjects) || !stored.gaps.same(&input.gaps) || !stored.coverage.same(&input.coverage) {
            return Err(invalid("projection source/domain closure differs"));
        }
        if snapshots {
            if stored.snapshots.len()!=1 {return Err(invalid("projection snapshot domain differs"));}
            let header=stored.snapshots.iter().next().expect("one snapshot");
            let graph=snapshot::hydrate(header,&assessment,&stored.chunks,budget)?;
            graph.matches(&input)?;
            if stored.chunks.iter().any(|chunk|chunk.snapshot!=header.id()){return Err(invalid("foreign projection chunk"));}
        }
        Ok(())
    }
    pub fn validate_graph(&self,key:ProjectionKey,graph:&snapshot::MaterializedGraph,budget:&ResourceBudget)->Result<(),ModelError>{
        if !self.contains(key){return Err(invalid("advertised projection has no actual input/context collection"));}
        let input=describe_indexed(self.data,&self.index,self.metadata,key,budget)?;
        graph.matches(&input)
    }
    pub fn produce(&self,key:ProjectionKey,budget:&ResourceBudget)->Result<ProjectionOutput,ModelError>{
        if !self.keys.keys.contains(&key){return Err(invalid("projection has no input/context collection"));}
        let input=describe_indexed(self.data,&self.index,self.metadata,key,budget)?;
        encode_input(&input,budget)
    }
}
pub struct ProjectionKeys {
    keys: ChargedSet<ProjectionKey>,
    _charge: StateCharge,
}
impl ProjectionKeys {
    pub fn iter(&self) -> impl Iterator<Item = ProjectionKey> + '_ {
        self.keys.iter().copied()
    }
}
pub fn describe(
    data: &ProjectionData,
    key: ProjectionKey,
    budget: &ResourceBudget,
) -> Result<ProjectionInput, ModelError> {
    if !data.keys(budget)?.keys.contains(&key) {
        return Err(invalid("projection has no input/context collection"));
    }
    describe_indexed(data, &Index::new(data, budget)?, data, key, budget)
}
fn describe_indexed(
    data: &ProjectionData,
    index: &Index<'_>,
    metadata: &impl Metadata,
    key: ProjectionKey,
    budget: &ResourceBudget,
) -> Result<ProjectionInput, ModelError> {
    let spec = ProjectionSpec::builtin(key.name);
    let mut out = ProjectionInput {
        key,
        vertices: Rows::new(budget),
        arcs: Default::default(),
        subjects: Rows::new(budget),
        gaps: Rows::new(budget),
        coverage: Rows::new(budget),
        charge: StateCharge::new(budget, "projection-arcs"),
    };
    for entity in data.refs.iter() {
        if spec.accepts(entity) && data.entity_in(metadata, entity, key)? {
            out.vertices.insert(entity.clone())?;
        }
    }
    // Universe construction precedes policy/arc selection. Preserve source isolates plus every
    // explicitly resolved destination of this input/context, including acquired foreign inputs.
    // A missing canonical EntityRef refuses; this never manufactures an endpoint identity.
    let mut include = |entity: Id<EntityRef>| -> Result<(), ModelError> {
        let entity = need(&data.refs, entity)?;
        if spec.accepts(entity) {
            out.vertices.insert(entity.clone())?;
        }
        Ok(())
    };
    match key.name {
        ProjectionName::CallableInvocation | ProjectionName::DefinitionContainment => {
            for event in data.events.iter() {
                if event.context == key.context && data.occurrence_input(metadata, event.site)? == key.input {
                    for alternative in index.alternatives.get(&event.id()).into_iter().flatten() {
                        if let Some(entity) = alternative.entity {
                            include(entity)?;
                        }
                    }
                }
            }
        }
        ProjectionName::ImportReference => {
            for assessment in data.import_assessments.iter() {
                if data.qualified(metadata,
                    metadata.import_frame(assessment.observation)?.0,
                    key,
                )? {
                    for candidate in index.imports.get(&assessment.id()).into_iter().flatten() {
                        if let Some(module) = metadata.acquired_module(candidate.module)?
                        {
                            include(EntityRef::Module { module }.id())?;
                        }
                    }
                }
            }
            for assessment in data.reference_assessments.iter() {
                if data.qualified(metadata,
                    metadata.reference_frame(assessment.reference)?.0,
                    key,
                )? {
                    for candidate in index.references.get(&assessment.id()).into_iter().flatten() {
                        if let ReferenceEntityTarget::Binding { entity, .. } =
                            need(&data.reference_targets, candidate.target)?
                        {
                            include(*entity)?;
                        }
                    }
                }
            }
        }
        ProjectionName::PublicExposure => {
            for exposure in data.exposures.iter() {
                if exposure.context == key.context
                    && metadata.artifact_input(metadata.module_source(exposure.access)?)?
                        == key.input
                {
                    for candidate in index.exposures.get(&exposure.id()).into_iter().flatten() {
                        if let Some(entity) = need(&data.resolutions, candidate.resolution)?.entity
                        {
                            include(entity)?;
                        }
                    }
                }
            }
        }
    }
    for family in spec.families() {
        let mut found = false;
        for row in metadata.coverages().filter(|c| {
            c.context == key.context
                && c.family == *family
                && data.scope_input(metadata, c.scope) == Some(key.input)
        }) {
            found = true;
            out.coverage.insert(ProjectionSourceCoverage {
                assessment: out.assessment().id(),
                coverage: row.id,
            })?;
            if row.status != CoverageStatus::CompleteUnderStatedModel {
                out.gap(
                    ProjectionGapSubject::Coverage { coverage: row.id() },
                    ProjectionGapReason::IncompleteCoverage,
                )?;
            }
        }
        if !found {
            let scope = CoverageScope::Input { input: key.input }.id();
            need(&data.scopes, scope)?;
            out.gap(
                ProjectionGapSubject::MissingCoverage {
                    scope,
                    family: *family,
                },
                ProjectionGapReason::MissingCoverage,
            )?;
        }
    }
    if matches!(
        key.name,
        ProjectionName::CallableInvocation | ProjectionName::DefinitionContainment
    ) {
        for event in data.events.iter() {
            if event.context != key.context || data.occurrence_input(metadata, event.site)? != key.input {
                continue;
            }
            let owner = need(&data.owners, event.owner)?.entity;
            let alternatives = index
                .alternatives
                .get(&event.id())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            if key.name == ProjectionName::CallableInvocation
                && alternatives
                    .iter()
                    .any(|a| index.invocations.contains(&a.id()))
            {
                let assessment = index
                    .assessments
                    .get(&event.id())
                    .ok_or_else(|| invalid("projection event assessment absent"))?;
                if !assessment.complete
                    || !assessment.exact
                    || !assessment.known_receivers
                    || assessment.unresolved
                    || assessment.dispatch
                    || assessment.disagreement
                {
                    out.gap(
                        ProjectionGapSubject::EventAssessment {
                            assessment: assessment.id(),
                        },
                        ProjectionGapReason::EventUncertainty,
                    )?;
                }
            }
            if alternatives.is_empty() {
                out.gap(
                    ProjectionGapSubject::Event { event: event.id() },
                    ProjectionGapReason::NoAlternative,
                )?;
            }
            for row in alternatives {
                let target = need(
                    &data.targets,
                    need(&data.alternative_sources, row.source)?.target(),
                )?;
                let subject = ProjectionGapSubject::Alternative {
                    alternative: row.id(),
                };
                if key.name == ProjectionName::CallableInvocation
                    && data
                        .dispatch_assessments
                        .iter()
                        .any(|a| a.event == event.id() && a.target == target.id() && a.open)
                {
                    out.gap(subject.clone(), ProjectionGapReason::OverrideDispatch)?;
                }
                let selected = if key.name == ProjectionName::CallableInvocation {
                    index.invocations.contains(&row.id())
                } else {
                    matches!(target.phase, CallPhase::Definition | CallPhase::Decorator)
                };
                if !selected {
                    out.gap(subject, ProjectionGapReason::OutsidePolicy)?;
                    continue;
                }
                if let Some(entity) = row.entity {
                    need(&data.refs, entity)?;
                    if out.vertices.get(owner).is_none() || out.vertices.get(entity).is_none() {
                        out.gap(subject, ProjectionGapReason::OutsideUniverse)?;
                        continue;
                    }
                    let id = if key.name == ProjectionName::CallableInvocation {
                        ArcId::Invocation(row.id())
                    } else {
                        ArcId::Definition(row.id())
                    };
                    out.arc(id, owner, entity)?;
                } else {
                    out.gap(
                        subject,
                        if row.status == ResolutionStatus::Ambiguous {
                            ProjectionGapReason::Ambiguous
                        } else {
                            ProjectionGapReason::Unresolved
                        },
                    )?;
                }
            }
        }
    }
    if key.name == ProjectionName::DefinitionContainment {
        for owner in data.owners.iter() {
            if data.occurrence_input(metadata, owner.occurrence)? == key.input {
                out.arc(
                    ArcId::Containment(owner.id()),
                    owner.entity,
                    EntityRef::Occurrence {
                        occurrence: owner.occurrence,
                    }
                    .id(),
                )?;
                // The source declaration's normalized lexical owner is the definition edge.
                // Ordinary containment keeps its own occurrence target and is never reinterpreted.
                if let Some(callable) = index.definitions.get(&owner.occurrence) {
                    let target = EntityRef::Callable {
                        callable: *callable,
                    }
                    .id();
                    need(&data.refs, target)?;
                    out.arc(ArcId::SourceDefinition(owner.id()), owner.entity, target)?;
                }
            }
        }
    }
    if key.name == ProjectionName::ImportReference {
        for assessment in data.import_assessments.iter() {
            let (qualification, alias) = metadata.import_frame(assessment.observation)?;
            if !data.qualified(metadata, qualification, key)? {
                continue;
            }
            let subject = ProjectionGapSubject::Import {
                assessment: assessment.id(),
            };
            if assessment.status != ResolutionStatus::Resolved {
                out.gap(subject.clone(), resolution_gap(assessment.status))?;
            }
            let source = EntityRef::Occurrence {
                occurrence: alias,
            }
            .id();
            for candidate in index.imports.get(&assessment.id()).into_iter().flatten() {
                match metadata.acquired_module(candidate.module)? {
                    Some(module) => {
                        let target = EntityRef::Module { module }.id();
                        need(&data.refs, target)?;
                        if out.vertices.get(target).is_some() {
                            out.arc(ArcId::Import(candidate.id()), source, target)?;
                        } else {
                            out.gap(subject.clone(), ProjectionGapReason::OutsideUniverse)?;
                        }
                    }
                    _ => out.gap(subject.clone(), ProjectionGapReason::ExternalModule)?,
                }
            }
        }
        for assessment in data.reference_assessments.iter() {
            let (qualification, read) = metadata.reference_frame(assessment.reference)?;
            if !data.qualified(metadata, qualification, key)? {
                continue;
            }
            let subject = ProjectionGapSubject::Reference {
                assessment: assessment.id(),
            };
            if assessment.status != ResolutionStatus::Resolved {
                out.gap(subject.clone(), resolution_gap(assessment.status))?;
            }
            let source = EntityRef::Occurrence {
                occurrence: read,
            }
            .id();
            for candidate in index.references.get(&assessment.id()).into_iter().flatten() {
                match need(&data.reference_targets, candidate.target)? {
                    ReferenceEntityTarget::Binding { entity, .. } => {
                        need(&data.refs, *entity)?;
                        if out.vertices.get(*entity).is_some() {
                            out.arc(ArcId::Reference(candidate.id()), source, *entity)?;
                        } else {
                            out.gap(subject.clone(), ProjectionGapReason::OutsideUniverse)?;
                        }
                    }
                    ReferenceEntityTarget::Builtin { .. } => {
                        out.gap(subject.clone(), ProjectionGapReason::Builtin)?
                    }
                    ReferenceEntityTarget::Unresolved { .. } => {
                        out.gap(subject.clone(), ProjectionGapReason::Unresolved)?
                    }
                }
            }
        }
    }
    if key.name == ProjectionName::PublicExposure {
        for exposure in data.exposures.iter() {
            if exposure.context != key.context
                || metadata.artifact_input(metadata.module_source(exposure.access)?)?
                    != key.input
            {
                continue;
            }
            let subject = ProjectionGapSubject::Exposure {
                exposure: exposure.id(),
            };
            if exposure.status != ResolutionStatus::Resolved {
                out.gap(subject.clone(), resolution_gap(exposure.status))?;
            }
            for candidate in index.exposures.get(&exposure.id()).into_iter().flatten() {
                let resolution = need(&data.resolutions, candidate.resolution)?;
                if let Some(entity) = resolution.entity {
                    need(&data.refs, entity)?;
                    if out.vertices.get(entity).is_some() {
                        out.arc(
                            ArcId::PublicExposure(candidate.id()),
                            EntityRef::Module {
                                module: exposure.access,
                            }
                            .id(),
                            entity,
                        )?;
                    } else {
                        out.gap(subject.clone(), ProjectionGapReason::OutsideUniverse)?;
                    }
                } else {
                    out.gap(subject.clone(), resolution_gap(resolution.status))?;
                }
            }
        }
    }
    Ok(out)
}
fn resolution_gap(status: ResolutionStatus) -> ProjectionGapReason {
    if status == ResolutionStatus::Ambiguous {
        ProjectionGapReason::Ambiguous
    } else {
        ProjectionGapReason::Unresolved
    }
}
/// Build snapshots once, sequentially, after canonical fact and normalization stages complete.
pub fn normalize(
    data: &ProjectionData,
    budget: &ResourceBudget,
) -> Result<ProjectionOutput, ModelError> {
    let mut out = ProjectionOutput::new(budget);
    let index = Index::new(data, budget)?;
    for key in data.keys(budget)?.iter() {
        let input = describe_indexed(data, &index, data, key, budget)?;
        let graph = snapshot::MaterializedGraph::build(&input, budget)?;
        let encoded = graph.encode(budget)?;
        let assessment = out.assessments.insert(input.assessment())?;
        for row in input.subjects.iter() {
            out.subjects.insert(row.clone())?;
        }
        for row in input.gaps.iter() {
            out.gaps.insert(row.clone())?;
        }
        for row in input.coverage.iter() {
            out.coverage.insert(row.clone())?;
        }
        let header = snapshot::header(assessment, encoded.bytes().len())?;
        let snapshot = out.snapshots.insert(header)?;
        for (ordinal, bytes) in encoded.bytes().chunks(snapshot::CHUNK_BYTES).enumerate() {
            out.chunks.insert(ProjectionSnapshotChunk {
                snapshot,
                ordinal: ordinal as i64,
                payload: EvidenceBytes(bytes.to_vec()),
            })?;
        }
    }
    Ok(out)
}
fn encode_input(input:&ProjectionInput,budget:&ResourceBudget)->Result<ProjectionOutput,ModelError>{
    let mut out=ProjectionOutput::new(budget);
    let graph=snapshot::MaterializedGraph::build(input,budget)?;
    let encoded=graph.encode(budget)?;
    let assessment=out.assessments.insert(input.assessment())?;
    for row in input.subjects.iter(){out.subjects.insert(row.clone())?;}
    for row in input.gaps.iter(){out.gaps.insert(row.clone())?;}
    for row in input.coverage.iter(){out.coverage.insert(row.clone())?;}
    let snapshot=out.snapshots.insert(snapshot::header(assessment,encoded.bytes().len())?)?;
    for (ordinal,bytes) in encoded.bytes().chunks(snapshot::CHUNK_BYTES).enumerate(){out.chunks.insert(ProjectionSnapshotChunk{snapshot,ordinal:ordinal as i64,payload:EvidenceBytes(bytes.to_vec())})?;}
    Ok(out)
}
/// Canonical projection meaning and lineage are portable independently of a serialized graph.
pub fn validate_canonical(
    data: &ProjectionData, stored: &ProjectionOutput, budget: &ResourceBudget,
) -> Result<(), ModelError> {
    validate_inner(data, stored, budget, false)
}
pub fn validate(
    data: &ProjectionData,
    stored: &ProjectionOutput,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    validate_inner(data, stored, budget, true)
}
fn validate_inner(
    data: &ProjectionData, stored: &ProjectionOutput, budget: &ResourceBudget,
    snapshots: bool,
) -> Result<(), ModelError> {
    let index = Index::new(data, budget)?;
    let mut expected = ProjectionOutput::new(budget);
    for key in data.keys(budget)?.iter() {
        let input = describe_indexed(data, &index, data, key, budget)?;
        let assessment = input.assessment();
        expected.assessments.insert(assessment.clone())?;
        for row in input.subjects.iter() {
            expected.subjects.insert(row.clone())?;
        }
        for row in input.gaps.iter() {
            expected.gaps.insert(row.clone())?;
        }
        for row in input.coverage.iter() {
            expected.coverage.insert(row.clone())?;
        }
        if snapshots {
        let header = stored
            .snapshots
            .iter()
            .find(|s| s.assessment == assessment.id())
            .ok_or_else(|| invalid("missing projection snapshot"))?;
        let graph = snapshot::hydrate(header, &assessment, &stored.chunks, budget)?;
        graph.matches(&input)?;
        expected.snapshots.insert(header.clone())?;
        }
    }
    if !stored.assessments.same(&expected.assessments)
        || !stored.subjects.same(&expected.subjects)
        || !stored.gaps.same(&expected.gaps)
        || !stored.coverage.same(&expected.coverage)
        || (snapshots && !stored.snapshots.same(&expected.snapshots))
    {
        return Err(invalid("projection source closure differs"));
    }
    for chunk in stored.chunks.iter().filter(|_| snapshots) {
        if stored.snapshots.get(chunk.snapshot).is_none() {
            return Err(invalid("orphan graph chunk"));
        }
    }
    Ok(())
}
fn validate_compact(data:&CompactProjectionData,stored:&ProjectionOutput,budget:&ResourceBudget,snapshots:bool)->Result<(),ModelError>{
    let prepared=data.prepare(budget)?;
    let mut expected=ProjectionOutput::new(budget);
    for key in prepared.keys(){
        let input=describe_indexed(prepared.data,&prepared.index,prepared.metadata,key,budget)?;
        let assessment=input.assessment();
        expected.assessments.insert(assessment.clone())?;
        for row in input.subjects.iter(){expected.subjects.insert(row.clone())?;}
        for row in input.gaps.iter(){expected.gaps.insert(row.clone())?;}
        for row in input.coverage.iter(){expected.coverage.insert(row.clone())?;}

    }
    if !stored.assessments.same(&expected.assessments)||!stored.subjects.same(&expected.subjects)||!stored.gaps.same(&expected.gaps)||!stored.coverage.same(&expected.coverage)
{return Err(invalid("projection source closure differs"));}
    if snapshots {
        let mut advertised=ChargedSet::default();
        let mut charge=StateCharge::new(budget,"projection-snapshot-keys");
        for header in stored.snapshots.iter(){
            let assessment=need(&stored.assessments,header.assessment)?;
            let key=ProjectionKey{input:assessment.input,context:assessment.context,name:assessment.projection};
            if !advertised.insert(&mut charge,key)?{return Err(invalid("ambiguous projection snapshot"));}
            let input=describe_indexed(prepared.data,&prepared.index,prepared.metadata,key,budget)?;
            let graph=snapshot::hydrate(header,assessment,&stored.chunks,budget)?;graph.matches(&input)?;
        }
    }
    if snapshots&&stored.chunks.iter().any(|chunk|stored.snapshots.get(chunk.snapshot).is_none()){return Err(invalid("orphan graph chunk"));}
    Ok(())
}
pub fn invariants() -> Vec<Invariant> {
    let mut canonical = ProjectionData::validation_inputs();
    canonical.extend(ProjectionOutput::validation_inputs().into_iter().filter(|input| {
        input.type_id() != std::any::TypeId::of::<ProjectionSnapshot>()
            && input.type_id() != std::any::TypeId::of::<ProjectionSnapshotChunk>()
    }));
    let mut all = canonical.clone();
    all.extend([
        ValidationInput::of::<ProjectionSnapshot>(&["id"]),
        ValidationInput::of::<ProjectionSnapshotChunk>(&["id"]),
    ]);
    vec![Invariant {
        revision: 1,
        name: "normalized_projection_canonical",
        purpose: InvariantPurpose::Admission,
        inputs: canonical,
        create: std::sync::Arc::new(|budget| Box::new(Check {
            data: CompactProjectionData::new(budget), output: ProjectionOutput::new(budget),
            snapshots: false, budget: budget.clone(),
        })),
    }, Invariant {
        revision: 1,
        name: "normalized_projection_closure",
        purpose: InvariantPurpose::Admission,
        inputs: all,
        create: std::sync::Arc::new(|budget| Box::new(Check {
            data: CompactProjectionData::new(budget), output: ProjectionOutput::new(budget),
            snapshots: true, budget: budget.clone(),
        })),
    }]
}
struct Check {
    snapshots: bool,
    data: CompactProjectionData,
    output: ProjectionOutput,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn projection_scope(&self)->Option<AdmissionScope>{Some(if self.snapshots{AdmissionScope::Snapshot}else{AdmissionScope::Canonical})}
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if !self.data.visit(relation, batch)? && !self.output.visit(relation, batch)? {
            return Err(invalid("undeclared projection input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        validate_compact(&self.data, &self.output, &self.budget, self.snapshots)
    }
}
pub fn stage(profile: stages::Profile) -> stages::Stage {
    let mut inputs = crate::domain::normalized::event_normalization::stage(profile).inputs;
    macro_rules! prior { ($($field:ident: $ty:ty,)*) => { $(inputs.push(stages::RelationUse::completed::<$ty>());)* }; }
    crate::normalized_event_outputs!(prior);
    inputs.extend(ProjectionData::stage_inputs());
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    stages::Stage {
        name: "normalize_projections",
        inputs: crate::domain::normalized::facts_stage_inputs(inputs),
        outputs: super::relations()
            .iter()
            .map(stages::RelationUse::of_relation)
            .collect(),
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: stages::Effect::Pure,
        code: crate::domain::normalized::policy_revision(),
        configuration: ContentHash::of(b"materialized-projections/v1"),
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["normalized_projection_closure"]
}

#[cfg(test)]
mod tests {
    use super::super::snapshot::*;
    use super::*;
    fn topology(reverse: bool) -> (ProjectionInput, ResourceBudget, Vec<Id<EntityRef>>) {
        let budget = ResourceBudget::fixed(32 << 20).unwrap();
        let input = InputRevision::from_entries(vec![]).unwrap();
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"graph"),
            environment_digest: ContentHash::of(b"graph"),
            lock_digest: None,
        };
        let source =
            SourceArtifact::from_bytes(input.id(), "graph.py".into(), &[b' '; 64]).unwrap();
        let key = ProjectionKey {
            input: input.id(),
            context: context.id(),
            name: ProjectionName::DefinitionContainment,
        };
        let mut out = ProjectionInput {
            key,
            vertices: Rows::new(&budget),
            arcs: Default::default(),
            subjects: Rows::new(&budget),
            gaps: Rows::new(&budget),
            coverage: Rows::new(&budget),
            charge: StateCharge::new(&budget, "graph-fixture"),
        };
        let occurrences: Vec<_> = (0..6)
            .map(|i| Occurrence {
                source: source.id(),
                start: i,
                end: i + 1,
                syntax_kind: SyntaxKind::ExprCall,
                role: OccurrenceRole::Call,
                structural_path: vec![i as i32],
            })
            .collect();
        let entities: Vec<_> = occurrences
            .iter()
            .map(|o| EntityRef::Occurrence { occurrence: o.id() })
            .collect();
        let ids = entities.iter().map(Record::id).collect::<Vec<_>>();
        let mut order = (0..6).collect::<Vec<_>>();
        if reverse {
            order.reverse();
        }
        for i in order {
            out.vertices.insert(entities[i].clone()).unwrap();
        }
        // Diamond, cycle, parallel edges, self-loop, plus two isolates. Edge identities are
        // canonical source relationships, independent of traversal indexes and insertion order.
        let mut arcs: Vec<_> = [(0, 1), (0, 2), (1, 3), (2, 3), (3, 0), (0, 1), (3, 3)]
            .into_iter()
            .enumerate()
            .collect();
        if reverse {
            arcs.reverse();
        }
        for (ordinal, (a, b)) in arcs {
            let occurrence = Occurrence {
                structural_path: vec![100 + ordinal as i32],
                ..occurrences[a].clone()
            };
            let owner = OccurrenceOwnership {
                occurrence: occurrence.id(),
                owner: occurrences[a].id(),
                entity: ids[a],
            };
            out.arc(ArcId::Containment(owner.id()), ids[a], ids[b])
                .unwrap();
        }
        (out, budget, ids)
    }
    #[test]
    fn native_borrow_preserves_sccs_parallel_arcs_isolates_and_reservation() {
        use petgraph::visit::{
            EdgeFiltered, EdgeRef, IntoEdgeReferences, IntoNeighbors, IntoNodeIdentifiers, Reversed,
        };
        let (input, budget, ids) = topology(false);
        let graph = MaterializedGraph::build(&input, &budget).unwrap();
        let encoded = graph.encode(&budget).unwrap();
        let restored =
            MaterializedGraph::decode(encoded.bytes(), input.key, 6, 7, &budget).unwrap();
        let before = budget.reserved();
        let expected_arcs = graph.arcs().collect::<Vec<_>>();
        for graph in [&graph, &restored] {
            let (mut components, arcs, incoming) = graph.with_native_graph(|view| {
                assert_eq!(budget.reserved(), before);
                let components = petgraph::algo::kosaraju_scc(view)
                    .into_iter()
                    .map(|nodes| {
                        let mut ids = nodes
                            .into_iter()
                            .map(|node| view.entity_id(node))
                            .collect::<Vec<_>>();
                        ids.sort();
                        ids
                    })
                    .collect::<Vec<_>>();
                let arcs = view
                    .edge_references()
                    .map(|edge| {
                        assert_eq!(view.arc_id(edge.id()), *edge.weight());
                        Arc {
                            id: *edge.weight(),
                            source: view.entity_id(edge.source()),
                            target: view.entity_id(edge.target()),
                        }
                    })
                    .collect::<Vec<_>>();
                let root = view
                    .node_identifiers()
                    .find(|node| view.entity_id(*node) == ids[1])
                    .unwrap();
                let reversed = Reversed(view);
                let incoming = reversed
                    .neighbors(root)
                    .map(|node| view.entity_id(node))
                    .collect::<Vec<_>>();
                let filtered = EdgeFiltered::from_fn(view, |edge| {
                    edge.weight().role() == EndpointRole::Invocation
                });
                assert_eq!((&filtered).edge_references().count(), 0);
                assert_eq!(petgraph::algo::kosaraju_scc(&filtered).len(), 6);
                (components, arcs, incoming)
            });
            components.sort();
            let mut cycle = ids[..4].to_vec();
            cycle.sort();
            let mut expected = vec![cycle, vec![ids[4]], vec![ids[5]]];
            expected.sort();
            assert_eq!(components, expected);
            assert_eq!(arcs, expected_arcs);
            assert_eq!(incoming, vec![ids[0], ids[0]]);
            assert_eq!(budget.reserved(), before);
        }
    }
    #[test]
    fn snapshot_roundtrip_preserves_topology_and_selector_keeps_intermediate_nodes() {
        let (input, budget, ids) = topology(false);
        let graph = MaterializedGraph::build(&input, &budget).unwrap();
        assert_eq!((graph.vertex_count(), graph.arc_count()), (6, 7));
        assert_eq!(graph.outgoing(ids[0]).unwrap().len(), 3);
        assert!(graph.outgoing(ids[4]).unwrap().is_empty());
        let selected = graph
            .reachable(
                ids[0],
                petgraph::Direction::Outgoing,
                None,
                |e| e.id() == ids[3],
                &budget,
            )
            .unwrap();
        assert_eq!(selected.as_slice(), &[ids[3]]);
        let reverse = graph
            .reachable(
                ids[3],
                petgraph::Direction::Incoming,
                Some(EndpointRole::Containment),
                |_| true,
                &budget,
            )
            .unwrap();
        assert_eq!(reverse.as_slice().len(), 4);
        let filtered = graph
            .reachable(
                ids[0],
                petgraph::Direction::Outgoing,
                Some(EndpointRole::Invocation),
                |_| true,
                &budget,
            )
            .unwrap();
        assert_eq!(filtered.as_slice(), &[ids[0]]);
        let bytes = graph.encode(&budget).unwrap();
        let restored = MaterializedGraph::decode(bytes.bytes(), input.key, 6, 7, &budget).unwrap();
        restored.matches(&input).unwrap();
        for id in &ids {
            assert_eq!(graph.outgoing(*id), restored.outgoing(*id));
            assert_eq!(graph.incoming(*id), restored.incoming(*id));
        }
        let (shuffled, _, _) = topology(true);
        assert_eq!(
            bytes.bytes(),
            MaterializedGraph::build(&shuffled, &budget)
                .unwrap()
                .encode(&budget)
                .unwrap()
                .bytes()
        );
        drop((
            input, graph, selected, reverse, filtered, bytes, restored, shuffled,
        ));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn malformed_versions_chunks_capacity_and_resources_refuse_without_leaks() {
        let (input, budget, _) = topology(false);
        let graph = MaterializedGraph::build(&input, &budget).unwrap();
        let encoded = graph.encode(&budget).unwrap();
        let bytes = encoded.bytes();
        assert!(
            MaterializedGraph::decode(&bytes[..bytes.len() - 1], input.key, 6, 7, &budget).is_err()
        );
        let mut trailing = bytes.to_vec();
        trailing.push(0);
        assert!(MaterializedGraph::decode(&trailing, input.key, 6, 7, &budget).is_err());
        let mut wrong = bytes.to_vec();
        wrong[0] = 4;
        assert!(MaterializedGraph::decode(&wrong, input.key, 6, 7, &budget).is_err());
        let key = ProjectionKey {
            name: ProjectionName::CallableInvocation,
            ..input.key
        };
        assert!(MaterializedGraph::decode(bytes, key, 6, 7, &budget).is_err());
        assert!(MaterializedGraph::decode(bytes, input.key, 5, 7, &budget).is_err());
        let tiny = ResourceBudget::fixed(64).unwrap();
        assert!(matches!(
            MaterializedGraph::build(&input, &tiny),
            Err(ModelError::Resource { .. })
        ));
        assert!(matches!(
            MaterializedGraph::decode(bytes, input.key, 6, 7, &tiny),
            Err(ModelError::Resource { .. })
        ));
        assert_eq!(tiny.reserved(), 0);
        assert!(check_capacity(u32::MAX as usize, 0).is_err());
        assert!(check_capacity(0, u32::MAX as usize).is_err());
        let assessment = input.assessment();
        let header = header(assessment.id(), bytes.len()).unwrap();
        let mut chunks = Rows::new(&budget);
        assert!(hydrate(&header, &assessment, &chunks, &budget).is_err());
        chunks
            .insert(ProjectionSnapshotChunk {
                snapshot: header.id(),
                ordinal: 1,
                payload: EvidenceBytes(bytes.to_vec()),
            })
            .unwrap();
        assert!(hydrate(&header, &assessment, &chunks, &budget).is_err());
    }
    #[test]
    fn empty_collection_has_all_named_snapshots_and_explicit_missing_availability() {
        let budget = ResourceBudget::fixed(8 << 20).unwrap();
        let mut data = ProjectionData::new(&budget);
        let (fixture, _, _) = topology(false);
        let key = fixture.key;
        let provider = Provider {
            tool: "empty".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"empty"),
        };
        let (run, _) = ProviderRun::new(
            provider.id(),
            key.context,
            key.input,
            ContentHash::of(b"empty"),
            [FactFamily::Calls],
        )
        .unwrap();
        data.runs.insert(run).unwrap();
        data.scopes
            .insert(CoverageScope::Input { input: key.input })
            .unwrap();
        let output = normalize(&data, &budget).unwrap();
        validate(&data, &output, &budget).unwrap();
        assert_eq!(output.snapshots.len(), 4);
        for row in output.assessments.iter() {
            assert_eq!((row.vertices, row.arcs), (0, 0));
            assert_eq!(row.availability, ProjectionAvailability::Partial);
            assert!(row.gaps > 0);
        }
        drop((data, output));
        assert_eq!(budget.reserved(), 0);
    }
    #[test]
    fn a_snapshot_spanning_multiple_bytea_chunks_hydrates_without_edge_reconstruction() {
        let (mut input, _, _) = topology(false);
        let budget = ResourceBudget::fixed(256 << 20).unwrap();
        input.vertices = Rows::new(&budget);
        input.arcs = Default::default();
        let source = SourceArtifact::from_bytes(input.key.input, "large.py".into(), b"").unwrap();
        for i in 0..70_000 {
            let occurrence = Occurrence {
                source: source.id(),
                start: i,
                end: i,
                syntax_kind: SyntaxKind::ExprName,
                role: OccurrenceRole::Read,
                structural_path: vec![i as i32],
            };
            input
                .vertices
                .insert(EntityRef::Occurrence {
                    occurrence: occurrence.id(),
                })
                .unwrap();
        }
        let graph = MaterializedGraph::build(&input, &budget).unwrap();
        let encoded = graph.encode(&budget).unwrap();
        assert!(encoded.bytes().len() > CHUNK_BYTES);
        let assessment = input.assessment();
        let header = header(assessment.id(), encoded.bytes().len()).unwrap();
        let mut chunks = Rows::new(&budget);
        for (ordinal, bytes) in encoded.bytes().chunks(CHUNK_BYTES).enumerate() {
            let row = ProjectionSnapshotChunk {
                snapshot: header.id(),
                ordinal: ordinal as i64,
                payload: EvidenceBytes(bytes.to_vec()),
            };
            chunks
                .decode(&ProjectionSnapshotChunk::encode(&[row]).unwrap())
                .unwrap();
        }
        hydrate(&header, &assessment, &chunks, &budget)
            .unwrap()
            .matches(&input)
            .unwrap();
        drop((input, graph, encoded, chunks));
        assert_eq!(budget.reserved(), 0);
    }
}

/// Canonical records require their portable named meaning, without old physical graph bytes.
pub(crate) fn canonical_invariants_refs() -> Vec<&'static str> {
    vec!["normalized_projection_canonical"]
}
#[cfg(test)]
mod canonical_admission_controls {
    use super::*;
    fn nominal<T>(byte: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new([byte;16].into_iter())).unwrap()
    }
    #[test]
    fn portable_metadata_has_no_serialized_graph_dependency_and_refuses_foreign_domain() {
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let data = ProjectionData::new(&budget);
        let mut output = ProjectionOutput::new(&budget);
        validate_canonical(&data, &output, &budget).unwrap();
        let descriptor = invariants().into_iter().find(|check| check.name == "normalized_projection_canonical").unwrap();
        assert!(!descriptor.inputs.iter().any(|input| input.type_id() == std::any::TypeId::of::<ProjectionSnapshotChunk>() || input.type_id() == std::any::TypeId::of::<ProjectionSnapshot>()));
        output.assessments.insert(ProjectionSourceAssessment { input: nominal(1), context: nominal(2), projection: ProjectionName::CallableInvocation,
            version: ProjectionSpec::VERSION, vertices: 0, arcs: 0, gaps: 0,
            availability: ProjectionAvailability::CompleteUnderStatedModel }).unwrap();
        assert!(validate_canonical(&data, &output, &budget).is_err());
    }
}
