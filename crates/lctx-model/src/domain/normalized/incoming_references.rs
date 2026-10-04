//! Incoming captured lexical name references. Nominal normalized targets, never spelling,
//! establish correspondence. This is not an external-use inventory or an execution count.
use crate::domain::{*, selection::classification::ClassificationData, resources::ResourceBudget,
    normalized::{entities::*, links::*, contract_comparison::ChargedResult}, serving::ProofReference};
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSearchScope {
    pub input: Id<input::InputRevision>,
    pub member: Id<catalog::CatalogMember>,
    pub parameter: Option<Id<ParameterEntity>>,
    pub artifacts: Vec<Id<source::SourceArtifact>>,
    pub contexts: Vec<Id<attribution::AnalysisContext>>,
    pub lexical_coverage: Vec<Id<attribution::ProviderCoverage>>,
    pub examined_name_references: u64,
    pub identity: ContentHash,
    pub unresolved_name_references: u64,
    pub unsupported_name_references: u64,
    /// Only final lexical name observations with supported normalized correspondence are represented.
    pub lexical_names_only: bool,
    /// External, dynamic, member and keyword-label users are outside this represented domain.
    pub external_consumers_unknown: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IncomingReference {
    pub reference: Id<lexical::ReferenceObservation>,
    pub characterization: Id<ReferenceBindingCharacterization>,
    pub candidate: Id<ReferenceEntityCandidate>,
    pub target: Id<EntityRef>,
    pub status: ResolutionStatus,
    pub context: Id<attribution::AnalysisContext>,
    pub artifact: Id<source::SourceArtifact>,
    pub occurrence: Id<source::Occurrence>,
    pub start: i64,
    pub end: i64,
    pub field: lexical::SyntaxField,
    pub call_target_syntax: bool,
    pub load: bool,
    pub typing: Option<bool>,
    pub typing_only_annotation: Option<bool>,
    pub runtime_annotation: Option<bool>,
    pub string_annotation: Option<bool>,
    pub type_checking: Option<bool>,
    pub proof: Vec<ProofReference>,
}
pub struct ReferenceResult { pub scope: ReferenceSearchScope, pub references: Vec<IncomingReference> }

pub fn member_entities(d: &ClassificationData, member: Id<catalog::CatalogMember>) -> BTreeSet<Id<EntityRef>> {
    let mut entities=BTreeSet::new();
    for callable in d.source.catalog.callables.iter().filter(|c| c.member==member) {
        if let Some(a)=d.source.core.assessments.get(callable.assessment) {
            entities.insert(EntityRef::Callable {callable:a.callable}.id());
        }
    }
    for class in d.source.catalog.classes.iter().filter(|c|c.member==member) {
        entities.insert(EntityRef::Class {class:class.class}.id());
    }
    entities
}
pub fn incoming(d: &ClassificationData, member: Id<catalog::CatalogMember>, parameter: Option<Id<ParameterEntity>>, budget: &ResourceBudget) -> Result<ChargedResult<ReferenceResult>,ModelError> {
    let selected=d.source.catalog.members.get(member).ok_or_else(|| ModelError::Invalid("reference member absent".into()))?;
    // Count every inspected inventory before allocating indexes or entity sets, including
    // member/formal ownership scans and the native support/coverage validation inputs.
    let total=[d.source.core.references.len(),d.source.core.reference_candidates.len(),
        d.facts.reference_characterizations.len(),d.source.core.artifacts.len(),
        d.source.catalog.callables.len(),d.source.catalog.classes.len(),d.source.core.assessments.len(),
        d.source.core.parameters.len(),d.source.core.parameter_links.len(),d.facts.signature_parameters.len(),
        d.source.core.variants.len(),d.source.core.reference_assessments.len(),d.source.core.reference_targets.len(),
        d.source.core.occurrences.len(),d.source.core.qualifications.len(),d.facts.native_contexts.len(),
        d.facts.native_bindings.len(),d.facts.native_context_supports.len(),d.facts.native_binding_supports.len(),
        d.facts.runs.len(),d.source.core.native_coverage.len()].into_iter().fold(0usize,usize::saturating_add);
    if total>100_000 { return Err(ModelError::Limit {owner:"incoming-references",limit:"input rows",observed:total,bound:100_000}) }
    let _scratch=budget.reserve("incoming-reference-indexes",total.saturating_add(1).saturating_mul(512))?;
    let mut charge=budget.reserve("incoming-reference-result",d.source.core.artifacts.len().saturating_mul(32).saturating_add(1024))?;
    let mut entities=member_entities(d,member);
    if let Some(parameter)=parameter {
        let signatures=d.source.core.variants.iter().filter(|v|v.callable.is_some_and(|c|entities.contains(&EntityRef::Callable {callable:c}.id()))).map(|v|v.signature).collect::<BTreeSet<_>>();
        if d.source.core.parameters.get(parameter).is_none() || !d.source.core.parameter_links.iter().filter(|l|l.entity==parameter)
            .filter_map(|l|d.facts.signature_parameters.get(l.parameter))
            .any(|p|signatures.contains(&p.signature)) {
            return Err(ModelError::Invalid("reference formal belongs to another member".into()));
        }
        entities.clear(); entities.insert(EntityRef::Parameter {parameter}.id());
    }
    let mut artifacts=d.source.core.artifacts.iter().filter(|a|a.input==selected.input).map(Record::id).collect::<Vec<_>>(); artifacts.sort();
    let mut key=KeySink::new("incoming-reference-captured-name-universe/v1"); selected.input.encode(&mut key); member.encode(&mut key); parameter.encode(&mut key);
    for artifact in &artifacts { artifact.encode(&mut key); }
    for reference in d.source.core.references.iter() { reference.content_digest().encode(&mut key); }
    let assessments=d.source.core.reference_assessments.iter().map(|a|(a.reference,a)).collect::<std::collections::BTreeMap<_,_>>();
    let mut characterizations=std::collections::BTreeMap::<_,Vec<_>>::new();
    for c in d.facts.reference_characterizations.iter() {characterizations.entry(c.reference).or_default().push(c);}
    let mut unresolved=0; let mut unsupported=0; let mut examined=0; let mut contexts=BTreeSet::new();let mut rows=Vec::new();
    // An admitted normalizer supplies native support association. Missing supported correspondence
    // remains in the scope remainder; it is not a negative target assertion.
    for reference in d.source.core.references.iter() {
        let Some(occurrence)=d.source.core.occurrences.get(reference.read) else { return Err(ModelError::Invalid("reference source absent".into())) };
        if !artifacts.contains(&occurrence.source) {continue}
        let Some(q)=d.source.core.qualifications.get(reference.qualification) else {return Err(ModelError::Invalid("reference qualification absent".into()))};
        examined+=1;contexts.insert(q.context);
        let Some(assessment)=assessments.get(&reference.id()).copied() else {unresolved+=1;continue};
        if assessment.status!=ResolutionStatus::Resolved {unresolved+=1;}
        let mut supported=false;
        for c in characterizations.get(&reference.id()).into_iter().flatten() {
            if !matches!(c.status,ResolutionStatus::Resolved|ResolutionStatus::Ambiguous) || c.status!=assessment.status {continue}
            let Some(candidate)=c.candidate.and_then(|id|d.source.core.reference_candidates.get(id)) else {continue};
            let Some(ReferenceEntityTarget::Binding {entity,event})=d.source.core.reference_targets.get(candidate.target) else {continue};
            let Some(native)=d.facts.native_contexts.get(c.native_context) else {continue};
            let Some(native_q)=d.source.core.qualifications.get(native.qualification) else {continue};
            if native_q.context!=q.context || native.subject!=reference.read || native.phase!=ruff::ContextPhase::FinalReference || native.reference_load!=Some(true) {continue}
            let Some(binding)=d.facts.native_bindings.get(c.binding) else {continue};
            let Some(binding_q)=d.source.core.qualifications.get(binding.qualification) else {continue};
            if binding_q.context!=q.context || binding.event!=*event || native.final_binding!=Some(*event) {continue}
            let Some(cs)=c.context_support.and_then(|id|d.facts.native_context_supports.get(id)) else {continue};
            let Some(bs)=c.support.and_then(|id|d.facts.native_binding_supports.get(id)) else {continue};
            if cs.assertion!=native.id() || bs.assertion!=binding.id() || candidate.assessment!=assessment.id() || cs.run!=bs.run || cs.surface!=bs.surface {continue}
            let Some(run)=d.facts.runs.get(cs.run) else {continue};
            if run.context!=q.context || run.input!=selected.input {continue}
            supported=true;
            if !entities.contains(entity) {continue}
            charge.try_resize(charge.size().saturating_add(4096))?;
            rows.push(IncomingReference {
                reference:reference.id(),characterization:c.id(),candidate:candidate.id(),target:*entity,status:assessment.status,
                context:q.context,artifact:occurrence.source,occurrence:reference.read,start:occurrence.start,end:occurrence.end,field:reference.field,
                call_target_syntax:reference.field==lexical::SyntaxField::Callee,load:true,
                typing:native.typing,typing_only_annotation:native.typing_only_annotation,runtime_annotation:native.runtime_annotation,string_annotation:native.string_annotation,type_checking:native.type_checking,
                proof:[derivation::RowRef::of(reference.id()),derivation::RowRef::of(assessment.id()),derivation::RowRef::of(candidate.id()),derivation::RowRef::of(candidate.target),derivation::RowRef::of(c.id()),derivation::RowRef::of(native.id()),derivation::RowRef::of(binding.id()),derivation::RowRef::of(cs.id()),derivation::RowRef::of(bs.id())].into_iter().map(ProofReference::from_canonical).collect(),
            });
        }
        if !supported {unsupported+=1;}
    }
    rows.sort_by_key(|r|(r.artifact,r.start,r.end,r.reference,r.characterization));
    rows.dedup_by_key(|r|(r.reference,r.characterization));
    let mut lexical_coverage=d.source.core.native_coverage.iter().filter(|c|c.family==attribution::FactFamily::Lexical && contexts.contains(&c.context) && c.run.and_then(|r|d.facts.runs.get(r)).is_some_and(|r|r.input==selected.input)).map(Record::id).collect::<Vec<_>>();lexical_coverage.sort();
    charge.try_resize(charge.size().saturating_add((contexts.len()+lexical_coverage.len())*64))?;
    for coverage in &lexical_coverage {coverage.encode(&mut key);}
    Ok(ChargedResult {value:ReferenceResult {scope:ReferenceSearchScope {input:selected.input,member,parameter,artifacts,contexts:contexts.into_iter().collect(),lexical_coverage,examined_name_references:examined,identity:key.finish(),unresolved_name_references:unresolved,unsupported_name_references:unsupported,lexical_names_only:true,external_consumers_unknown:true},references:rows},_charge:charge})
}

pub fn definition() -> ContentHash { ContentHash::of(include_bytes!("incoming_references.rs")) }
