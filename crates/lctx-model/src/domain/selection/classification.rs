//! Explicit metadata actually consumed by finite classification. This is not C2 replay input.
use super::build::{Data, Output};
use crate::domain::{normalized::Rows, resources::ResourceBudget, *};

fn copy<R: Record>(
    source: &Rows<R>,
    target: &mut Rows<R>,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    for row in source.iter() {
        let _temporary = b.reserve(
            "selection-projection-copy",
            size_of::<R>().saturating_add(row.heap_bytes()),
        )?;
        target.insert(row.clone())?;
    }
    Ok(())
}
// Each concrete semantic group declares its consumed rows once. Every storage/decoder/projection
// operation below expands from that inventory; fields retain the evaluator's existing paths.
macro_rules! group {
    ($name:ident,$source:ty,{$($field:ident:$row:ty,)*})=>{
        pub struct $name {$(pub $field:Rows<$row>,)*}
        impl $name {
            fn new(b:&ResourceBudget)->Self {Self{$($field:Rows::new(b),)*}}
            fn visit(&mut self,n:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if n==<$row>::NAME {self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
            fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$row>(&["id"]),)*]}
            fn project(source:&$source,b:&ResourceBudget)->Result<Self,ModelError> {let mut out=Self::new(b);$(copy(&source.$field,&mut out.$field,b)?;)*Ok(out)}
        }
    };
}
group!(Catalog, crate::domain::catalog::build::CatalogOutput, {
    members: crate::domain::catalog::CatalogMember,
    invocations: crate::domain::catalog::CatalogInvocation,
    callables: crate::domain::catalog::CatalogCallable,
    candidates: crate::domain::catalog::CatalogCandidate,
    exposures: crate::domain::catalog::CatalogExposure,
    options: crate::domain::catalog::CatalogOption,
    subjects: crate::domain::catalog::CatalogOptionSubject,
    defaults: crate::domain::catalog::CatalogDefault,
    paths: crate::domain::catalog::CatalogPath,
    aliases: crate::domain::catalog::CatalogAlias,
    classes: crate::domain::catalog::CatalogClass,
});
group!(Core, crate::domain::catalog::build::CatalogData, {
    bindings: crate::domain::lexical::BindingObservation,
    binding_events: crate::domain::lexical::BindingEvent,
    lexical_scopes: crate::domain::lexical::LexicalScope,
    occurrences: crate::domain::source::Occurrence,
    placements: crate::domain::syntax::SyntaxPlacement,
    references: crate::domain::lexical::ReferenceObservation,
    reference_assessments: crate::domain::normalized::links::ReferenceEntityAssessment,
    reference_candidates: crate::domain::normalized::links::ReferenceEntityCandidate,
    reference_targets: crate::domain::normalized::links::ReferenceEntityTarget,
    lexical_resolutions: crate::domain::lexical::LexicalResolution,
    resolutions: crate::domain::normalized::entities::SymbolEntityResolution,
    class_metadata: crate::domain::class_metadata::ClassMetadataObservation,
    modules: crate::domain::source::Module,
    slots: crate::domain::normalized::callables::SignatureSlot,
    slot_types: crate::domain::normalized::callables::SignatureSlotType,
    return_types: crate::domain::normalized::callables::SignatureReturnType,
    signature_types: crate::domain::types::SignatureTypeObservation,
    native_signatures: crate::domain::types::NativeSignatureObservation,
    fields: crate::domain::normalized::entities::FieldEntity,
    slot_entities: crate::domain::normalized::callables::SignatureSlotEntity,
    parameter_links: crate::domain::normalized::entities::ParameterEntityLink,
    parameters: crate::domain::normalized::entities::ParameterEntity,
    refs: crate::domain::normalized::entities::EntityRef,
    source_callables: crate::domain::normalized::entities::CallableEntity,
    declarations: crate::domain::syntax::DeclarationObservation,
    exposures: crate::domain::normalized::entities::PublicExposure,
    assessments: crate::domain::normalized::callables::EffectiveCallableAssessment,
    variants: crate::domain::normalized::callables::SignatureVariant,
    qualifications: crate::domain::assertion::AssertionQualification,
    ownership: crate::domain::normalized::entities::OccurrenceOwnership,
    field_links: crate::domain::normalized::entities::FieldEntityLink,
    field_observations: crate::domain::types::RecordFieldObservation,
    entity_candidates: crate::domain::normalized::entities::SymbolEntityCandidate,
    symbols: crate::domain::calls::ProviderSymbol,
    provider_modules: crate::domain::calls::ProviderModule,
});
group!(SourceFacts, crate::domain::catalog::evidence::build::EvidenceFacts, {
    alternatives: crate::domain::normalized::events::NormalizedCallAlternative,
    deployment: crate::domain::deployment::DeploymentObservation,
});
group!(Evidence, crate::domain::catalog::evidence::build::EvidenceOutput, {
    accesses: crate::domain::catalog::evidence::FieldAccessAssessment,
    associations: crate::domain::catalog::evidence::ScenarioAssociation,
    scenarios: crate::domain::catalog::evidence::CatalogScenario,
    deployments: crate::domain::catalog::evidence::CatalogDeployment,
});
group!(Facts, crate::domain::selection::build::Facts, {
    decorators: crate::domain::syntax::DeclarationDecorator,
    decorator_supports: crate::domain::syntax::DeclarationDecoratorSupport,
    binding_supports: crate::domain::lexical::BindingSupport,
    declaration_supports: crate::domain::syntax::DeclarationSupport,
    metadata_supports: crate::domain::class_metadata::ClassMetadataSupport,
    native_signature_supports: crate::domain::types::NativeSignatureSupport,
    type_supports: crate::domain::types::TypeSupport,
    exception_outcomes: crate::domain::execution::summary_exceptions::SummaryExceptionOutcome,
    shapes: crate::domain::calls::ParameterShape,
    signature_parameters: crate::domain::calls::SignatureParameter,
    signatures: crate::domain::calls::Signature,
    type_observations: crate::domain::types::TypeObservation,
    call_syntax: crate::domain::calls::CallSyntax,
    generic_specializations: crate::domain::types::GenericSpecializationObservation,
    signature_subjects: crate::domain::types::SignatureTypeSubject,
    type_sequence_headers: crate::domain::types::TypeSequence,
    type_callable_lists: crate::domain::types::CallableParameterList,
    type_callable_slots: crate::domain::types::CallableParameter,
    type_dict_lists: crate::domain::types::TypedDictFieldList,
    type_dict_fields: crate::domain::types::TypedDictField,
    type_variables: crate::domain::types::TypeVariable,
    type_terms: crate::domain::types::TypeTerm,
    type_sequences: crate::domain::types::TypeSequenceMember,
    literals: crate::domain::value::Literal,
    packages: crate::domain::input::Package,
    releases: crate::domain::input::Release,
});
pub struct Source {
    pub catalog: Catalog,
    pub core: Core,
    pub facts: SourceFacts,
}
/// All classification rows, including negative and absence lookups; no bodies or documents.
pub struct ClassificationData {
    pub source: Source,
    pub evidence: Evidence,
    pub facts: Facts,
}
impl ClassificationData {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            source: Source {
                catalog: Catalog::new(b),
                core: Core::new(b),
                facts: SourceFacts::new(b),
            },
            evidence: Evidence::new(b),
            facts: Facts::new(b),
        }
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut rows = Catalog::inputs();
        rows.extend(Core::inputs());
        rows.extend(SourceFacts::inputs());
        rows.extend(Evidence::inputs());
        rows.extend(Facts::inputs());
        rows.sort_by_key(|r| r.name());
        rows
    }
    pub fn visit(&mut self, n: &str, batch: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        Ok(self.source.catalog.visit(n, batch)?
            || self.source.core.visit(n, batch)?
            || self.source.facts.visit(n, batch)?
            || self.evidence.visit(n, batch)?
            || self.facts.visit(n, batch)?)
    }
    pub fn project(source: &Data, b: &ResourceBudget) -> Result<Self, ModelError> {
        Ok(Self {
            source: Source {
                catalog: Catalog::project(&source.source.catalog, b)?,
                core: Core::project(&source.source.core, b)?,
                facts: SourceFacts::project(&source.source.facts, b)?,
            },
            evidence: Evidence::project(&source.evidence, b)?,
            facts: Facts::project(&source.facts, b)?,
        })
    }
}
pub(super) fn copy_output(source: &Output, b: &ResourceBudget) -> Result<Output, ModelError> {
    let mut out = Output::new(b);
    macro_rules! rows {($($f:ident:$ty:ty,)*)=>{$(copy(&source.$f,&mut out.$f,b)?;)*};}
    crate::catalog_selection_outputs!(rows);
    Ok(out)
}
impl ClassificationData {
    /// Required metadata references only: optional absence remains meaningful to evaluation.
    pub(super) fn validate_references(&self) -> Result<(), ModelError> {
        use super::build::need;
        use crate::domain::{catalog::CatalogOptionSubject, normalized::entities::EntityRef};
        let c = &self.source.catalog;
        let n = &self.source.core;
        let f = &self.facts;
        for r in c.members.iter() {
            need(&n.modules, r.access)?;
        }
        for r in c.exposures.iter() {
            need(&c.members, r.member)?;
            need(&n.exposures, r.exposure)?;
        }
        for r in c.candidates.iter() {
            need(&c.exposures, r.exposure)?;
            if let Some(id) = r.entity {
                need(&n.entity_candidates, id)?;
            }
            if let Some(id) = r.path {
                need(&c.paths, id)?;
            }
            if let Some(id) = r.alias {
                need(&c.aliases, id)?;
            }
        }
        for r in c.paths.iter() {
            need(&c.candidates, r.parent)?;
            need(&n.refs, r.entity)?;
        }
        for r in c.aliases.iter() {
            need(&n.refs, r.entity)?;
        }
        for r in c.callables.iter() {
            need(&c.members, r.member)?;
            need(&c.candidates, r.candidate)?;
            need(&n.assessments, r.assessment)?;
        }
        for r in c.invocations.iter() {
            need(&c.callables, r.callable)?;
            need(&n.variants, r.variant)?;
        }
        for r in c.options.iter() {
            need(&c.members, r.member)?;
            need(&c.subjects, r.subject)?;
            need(&c.defaults, r.default)?;
        }
        for r in c.subjects.iter() {
            match r {
                CatalogOptionSubject::Parameter { slot } => {
                    need(&n.slots, *slot)?;
                }
                CatalogOptionSubject::Field { field } => {
                    need(&n.fields, *field)?;
                }
                _ => {}
            }
        }
        for r in c.defaults.iter() {
            if let crate::domain::catalog::CatalogDefault::Literal { literal } = r {
                need(&f.literals, *literal)?;
            }
        }
        for r in c.classes.iter() {
            need(&c.members, r.member)?;
            need(&c.candidates, r.candidate)?;
        }
        for r in n.slots.iter() {
            need(&n.variants, r.variant)?;
            need(&f.signature_parameters, r.parameter)?;
        }
        for r in n.slot_entities.iter() {
            need(&n.slots, r.slot)?;
            need(&n.parameter_links, r.link)?;
        }
        for r in n.parameter_links.iter() {
            need(&n.parameters, r.entity)?;
        }
        for r in n.variants.iter() {
            need(&f.signatures, r.signature)?;
        }
        for r in n.field_links.iter() {
            need(&n.fields, r.field)?;
            need(&n.field_observations, r.observation)?;
        }
        for r in n.field_observations.iter() {
            need(&n.qualifications, r.qualification)?;
            need(&f.type_terms, r.term)?;
        }
        for r in n.entity_candidates.iter() {
            need(&n.refs, r.entity)?;
        }
        for r in n.refs.iter() {
            match r {
                EntityRef::Callable { .. } => {}
                EntityRef::Field { field } => {
                    need(&n.fields, *field)?;
                }
                _ => {}
            }
        }
        for r in n.symbols.iter() {
            need(&n.provider_modules, r.module)?;
        }
        for r in n.provider_modules.iter() {
            if let calls::ProviderModule::Acquired { module } = r {
                need(&n.modules, *module)?;
            }
        }
        for r in f.signature_parameters.iter() {
            need(&f.shapes, r.shape)?;
        }
        for r in f.type_observations.iter() {
            need(&f.type_terms, r.term)?;
            need(&n.qualifications, r.qualification)?;
        }
        for r in f.type_sequences.iter() {
            need(&f.type_terms, r.child)?;
        }
        for r in f.type_terms.iter() {
            match r {
                types::TypeTerm::ClassInstance { class, .. }
                | types::TypeTerm::ClassObject { class }
                | types::TypeTerm::TypedDict { class, .. } => {
                    need(&n.symbols, *class)?;
                }
                types::TypeTerm::Literal { value } => {
                    need(&f.literals, *value)?;
                }
                _ => {}
            }
        }
        for r in f.releases.iter() {
            need(&f.packages, r.package)?;
        }
        for r in self.evidence.accesses.iter() {
            need(&c.options, r.option)?;
            if r.applicability == crate::domain::normalized::callables::Knowledge::Known {
                need(&n.ownership, r.owner)?;
            }
            need(&n.qualifications, r.qualification)?;
        }
        for r in self.evidence.associations.iter() {
            need(&c.members, r.member)?;
            need(&self.evidence.scenarios, r.scenario)?;
            if r.basis == crate::domain::catalog::evidence::AssociationBasis::ResolvedTarget {
                need(&self.source.facts.alternatives, r.alternative)?;
            }
            need(&n.qualifications, r.qualification)?;
        }
        for r in self.evidence.deployments.iter() {
            need(&self.source.facts.deployment, r.observation)?;
        }
        Ok(())
    }
    pub(super) fn validate_context(&self, c: &super::Context) -> Result<(), ModelError> {
        use super::{
            Context,
            build::{invalid, need},
        };
        if let Some(m) = c.member() {
            need(&self.source.catalog.members, m)?;
        }
        match c {
            Context::Binding {
                candidate, member, ..
            } => {
                let r = need(&self.source.catalog.candidates, *candidate)?;
                if need(&self.source.catalog.exposures, r.exposure)?.member != *member {
                    return Err(invalid("binding context has foreign candidate"));
                }
            }
            Context::Signature {
                candidate,
                invocation,
                member,
                analysis,
            } => {
                let i = need(&self.source.catalog.invocations, *invocation)?;
                let callable = need(&self.source.catalog.callables, i.callable)?;
                if callable.member != *member
                    || callable.candidate != *candidate
                    || need(&self.source.core.assessments, callable.assessment)?.context
                        != *analysis
                {
                    return Err(invalid("signature context crosses callable roles"));
                }
            }
            Context::Scenario { scenario, .. } => {
                need(&self.evidence.scenarios, *scenario)?;
            }
            Context::Release { release, .. } => {
                need(&self.facts.releases, *release)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn validate_witness(
        &self,
        c: &super::Context,
        w: &super::Witness,
    ) -> Result<(), ModelError> {
        use super::{
            Context, Witness,
            build::{invalid, need},
        };
        match w {
            Witness::LexicalDefinition{observation,support}=>{let row=need(&self.source.core.bindings,*observation)?;let support=need(&self.facts.binding_supports,*support)?;if support.assertion!=row.id()||need(&self.source.core.qualifications,row.qualification)?.context!=c.analysis(){return Err(invalid("lexical definition witness crosses observation or context"));}}
            Witness::SourceCharacterization{observation,support}=>{
                let row=need(&self.source.core.declarations,*observation)?;let support=need(&self.facts.declaration_supports,*support)?;
                if support.assertion!=row.id()||need(&self.source.core.qualifications,row.qualification)?.context!=c.analysis(){return Err(invalid("source characterization witness crosses observation or context"));}
            }
            Witness::ClassMetadata{observation,support}=>{
                let row=need(&self.source.core.class_metadata,*observation)?;let support=need(&self.facts.metadata_supports,*support)?;
                if support.assertion!=row.id()||need(&self.source.core.qualifications,row.qualification)?.context!=c.analysis(){return Err(invalid("class metadata witness crosses observation or context"));}
            }
            Witness::NativeCallableMetadata{observation,support}=>{
                let row=need(&self.source.core.native_signatures,*observation)?;let support=need(&self.facts.native_signature_supports,*support)?;
                let Context::Signature{invocation,..}=c else{return Err(invalid("native metadata witness lacks signature context"));};
                let variant=need(&self.source.core.variants,need(&self.source.catalog.invocations,*invocation)?.variant)?;
                if support.assertion!=row.id()||variant.native!=Some(row.id())||need(&self.source.core.qualifications,row.qualification)?.context!=c.analysis(){return Err(invalid("native metadata witness crosses signature or context"));}
            }
            Witness::RaisedType{observation,support}=>{
                let row=need(&self.facts.type_observations,*observation)?;let support=need(&self.facts.type_supports,*support)?;
                if row.role!=types::TypeRole::Raised||support.assertion!=row.id()||need(&self.source.core.qualifications,row.qualification)?.context!=c.analysis(){return Err(invalid("raised typing witness crosses observation or context"));}
            }
            Witness::ResolvedDecorator{observation,support,assessment,candidate}=>{
                let row=need(&self.facts.decorators,*observation)?;let support=need(&self.facts.decorator_supports,*support)?;
                need(&self.source.core.reference_assessments,*assessment)?;let candidate=need(&self.source.core.reference_candidates,*candidate)?;
                if support.assertion!=row.id()||candidate.assessment!=*assessment||need(&self.source.core.qualifications,row.qualification)?.context!=c.analysis(){return Err(invalid("decorator witness crosses selected identity or context"));}
            }
            Witness::SummaryException { outcome } => {
                let result = need(&self.facts.exception_outcomes, *outcome)?;
                let q = need(&self.source.core.qualifications, result.qualification)?;
                let Context::Binding { member, candidate, analysis } = c else { return Err(invalid("exception witness lacks source binding context")); };
                let candidate = need(&self.source.catalog.candidates, *candidate)?;
                let owner = if let Some(path) = candidate.path { Some(need(&self.source.catalog.paths, path)?.entity) }
                    else if let Some(alias) = candidate.alias { Some(need(&self.source.catalog.aliases, alias)?.entity) }
                    else { candidate.entity.map(|id| need(&self.source.core.entity_candidates, id).map(|row| row.entity)).transpose()? };
                if result.context != *analysis || q.context != result.context || Some(result.owner) != owner
                    || result.input != need(&self.source.catalog.members, *member)?.input {
                    return Err(invalid("exception summary witness crosses source/context/input"));
                }
            }
            Witness::Candidate { candidate } => {
                need(&self.source.catalog.candidates, *candidate)?;
                if !matches!(c,Context::Binding{candidate:id,..} if id==candidate) {
                    return Err(invalid("candidate witness has foreign context role"));
                }
            }
            Witness::Invocation { invocation } => {
                need(&self.source.catalog.invocations, *invocation)?;
                if !matches!(c,Context::Signature{invocation:id,..} if id==invocation) {
                    return Err(invalid("invocation witness has foreign context role"));
                }
            }
            Witness::Option { option } => {
                let r = need(&self.source.catalog.options, *option)?;
                if c.member() != Some(r.member) {
                    return Err(invalid("option witness crosses member"));
                }
            }
            Witness::Association { association } => {
                let r = need(&self.evidence.associations, *association)?;
                if !matches!(c,Context::Scenario{scenario,member,..} if *scenario==r.scenario && *member==r.member)
                {
                    return Err(invalid("association witness crosses scenario"));
                }
            }
            Witness::TypeObservation { observation } => {
                let r = need(&self.facts.type_observations, *observation)?;
                if need(&self.source.core.qualifications, r.qualification)?.context != c.analysis()
                {
                    return Err(invalid("type witness crosses analysis"));
                }
            }
            Witness::GenericSpecialization { observation } => {
                let row=need(&self.facts.generic_specializations,*observation)?;
                if need(&self.source.core.qualifications,row.qualification)?.context!=c.analysis() {return Err(invalid("specialization witness crosses analysis"));}
                let Context::Signature {invocation,..}=c else {return Err(invalid("specialization witness lacks signature context"));};
                let variant=need(&self.source.core.variants,need(&self.source.catalog.invocations,*invocation)?.variant)?;
                if variant.native!=Some(row.declaration) {return Err(invalid("specialization witness crosses native declaration"));}
            }
            Witness::SignatureTypeObservation { observation } => {
                let row = need(&self.source.core.signature_types, *observation)?;
                if need(&self.source.core.qualifications, row.qualification)?.context != c.analysis() { return Err(invalid("typed port witness crosses analysis")); }
                let Context::Signature { invocation, .. } = c else { return Err(invalid("typed port witness lacks signature context")); };
                let variant = need(&self.source.catalog.invocations, *invocation)?.variant;
                if !self.source.core.return_types.iter().any(|r| r.variant == variant && r.observation == *observation)
                    && !self.source.core.slot_types.iter().any(|r| r.observation == *observation && self.source.core.slots.get(r.slot).is_some_and(|s| s.variant == variant)) { return Err(invalid("typed port witness crosses signature")); }
            }
            Witness::SignatureSlot { slot } => {
                let r = need(&self.source.core.slots, *slot)?;
                if let Context::Signature { invocation, .. } = c {
                    if need(&self.source.catalog.invocations, *invocation)?.variant != r.variant {
                        return Err(invalid("slot witness crosses signature"));
                    }
                } else {
                    return Err(invalid("slot witness lacks signature context"));
                }
            }
            Witness::FieldAccess { assessment } => {
                need(&self.evidence.accesses, *assessment)?;
            }
            Witness::Qualification { qualification } => {
                need(&self.source.core.qualifications, *qualification)?;
            }
            Witness::Deployment { deployment } => {
                need(&self.evidence.deployments, *deployment)?;
            }
            Witness::Member { member } => {
                need(&self.source.catalog.members, *member)?;
                if c.member() != Some(*member) {
                    return Err(invalid("member witness crosses context"));
                }
            }
            Witness::NativeField { observation } => {
                need(&self.source.core.field_observations, *observation)?;
            }
            _ => {}
        }
        Ok(())
    }
}
