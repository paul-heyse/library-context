//! Authored prose conclusions. This owner proves exact documentary extraction, never behavior.
use crate::domain::{
    analysis::{
        self,
        native::NativeAssertionPremise,
        policy::EvidenceStatus,
        support::{DerivedEvidence, SourceFacts},
    },
    artifact::{ARTIFACT_CHUNK_BYTES, ArtifactChunkKey},
    assertion::{Approximation, AssertionQualification},
    attribution::{Fidelity, Modality},
    catalog::*,
    normalized::{Rows, entities::*},
    resources::{Reservation, ResourceBudget},
    source::*,
    syntax::*,
    value::Literal,
    *,
};
use crate::{Domain, DomainCode, DomainSum};
use unicode_segmentation::UnicodeSegmentation;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum DocumentaryBoundaryReason {
    NoSourceDeclaration = 0,
    NoDocstring = 1,
    NativeEvidenceUnavailable = 2,
    UnsupportedLiteralMapping = 3,
    EmptySummary = 4,
    NoLeadMention = 5,
    ExcludedChangelog = 6,
    ForeignContext = 7,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "synthesis_documentary_boundaries")]
pub struct DocumentaryBoundary {
    #[model(key)]
    pub member: Id<CatalogMemberInvocation>,
    #[model(key)]
    pub candidate: Option<Id<CatalogCandidate>>,
    #[model(key)]
    pub association: Option<Id<evidence::DocumentAssociation>>,
    #[model(key)]
    pub reason: DocumentaryBoundaryReason,
}
/// The earlier original anchor remains the authority; these are derived excerpt operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum,serde::Serialize,serde::Deserialize)]
#[model(name = "synthesis_prose_sources")]
pub enum ProseSource {
    #[model(code = 0)]
    Occurrence { occurrence: Id<Occurrence> },
    #[model(code = 1)]
    Span {
        span: assertion::EvidenceSourceSpanId,
    },
    #[model(code = 2)]
    Literal {
        occurrence: Id<Occurrence>,
        literal: Id<Literal>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Domain,serde::Serialize,serde::Deserialize)]
#[model(name="synthesis_prose_slices",validate=validate_slice)]
pub struct ProseSlice {
    #[model(key)]
    pub source: Id<ProseSource>,
    #[model(key)]
    pub start: i64,
    #[model(key)]
    pub end: i64,
}
fn validate_slice(r: &ProseSlice) -> Result<(), ModelError> {
    if r.start < 0 || r.end < r.start {
        return Err(invalid("invalid documentary relative byte slice"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum,serde::Serialize,serde::Deserialize)]
#[model(
    name = "synthesis_documentary_sources",
    rule = "sourced_authored_prose"
)]
pub enum DocumentarySource {
    #[model(code = 0)]
    Literal {
        member: Id<CatalogMemberInvocation>,
        candidate: Id<CatalogCandidate>,
        #[model(premise)]
        declaration: Id<NativeAssertionPremise>,
        #[model(premise)]
        placement: Id<NativeAssertionPremise>,
        #[model(premise)]
        detail: Id<NativeAssertionPremise>,
        subject: Id<EntityRef>,
        docstring: Id<Occurrence>,
        literal: Id<Literal>,
        source_input: Id<crate::domain::input::InputRevision>,
        source_qualification: Id<AssertionQualification>,
    },
    #[model(code = 1)]
    Passage {
        member: Id<CatalogMemberInvocation>,
        association: Id<evidence::DocumentAssociation>,
        #[model(premise)]
        passage: Id<NativeAssertionPremise>,
        #[model(premise)]
        mention: Id<NativeAssertionPremise>,
        source_input: Id<crate::domain::input::InputRevision>,
        source_qualification: Id<AssertionQualification>,
    },
    #[model(code = 2)]
    Component {
        member: Id<CatalogMemberInvocation>,
        association: Id<evidence::DocumentAssociation>,
        component: Id<documents::DocumentComponentObservation>,
        #[model(premise)]
        premise: Id<NativeAssertionPremise>,
        #[model(premise)]
        scope: Id<NativeAssertionPremise>,
        #[model(premise)]
        field: Option<Id<NativeAssertionPremise>>,
        #[model(premise)]
        title: Option<Id<NativeAssertionPremise>>,
        option: Option<Id<CatalogOption>>,
        source_option: Option<Id<CatalogOption>>,
        role: super::documentary_templates::ComponentRole,
        source_input: Id<input::InputRevision>,
        source_qualification: Id<AssertionQualification>,
    },
}
impl DocumentarySource {
    pub fn member(&self) -> Id<CatalogMemberInvocation> {
        match self {
            Self::Literal { member, .. }
            | Self::Passage { member, .. }
            | Self::Component { member, .. } => *member,
        }
    }
    pub fn subject(&self) -> Option<Id<EntityRef>> {
        match self {
            Self::Literal { subject, .. } => Some(*subject),
            Self::Passage { .. } | Self::Component { .. } => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="synthesis_documentary_conclusions",rule="sourced_authored_prose",invariant_refs=invariants_refs,semantic_source=include_bytes!("documentary.rs"))]
pub struct DocumentaryConclusion {
    #[model(key, premise)]
    pub source: Id<DocumentarySource>,
    pub prose: Id<ProseSlice>,
    pub excerpt: Id<ProseSlice>,
    pub excerpt_digest: ContentHash,
    qualification: Id<AssertionQualification>,
    status: EvidenceStatus,
}
impl DocumentaryConclusion {
    pub(super) fn authored(
        source: Id<DocumentarySource>,
        prose: Id<ProseSlice>,
        excerpt: Id<ProseSlice>,
        excerpt_digest: ContentHash,
        qualification: Id<AssertionQualification>,
        status: EvidenceStatus,
    ) -> Self {
        Self {
            source,
            prose,
            excerpt,
            excerpt_digest,
            qualification,
            status,
        }
    }
    pub fn status(&self) -> EvidenceStatus {
        self.status
    }
    pub fn qualification(&self) -> Id<AssertionQualification> {
        self.qualification
    }
}
impl analysis::support::sealed::DerivedEvidence for DocumentaryConclusion {}
impl DerivedEvidence for DocumentaryConclusion {
    fn source_facts(&self) -> SourceFacts {
        SourceFacts {
            qualification: self.qualification,
            status: self.status,
            heuristic: false,
        }
    }
}
#[macro_export]
macro_rules! synthesis_documentary_inputs {($m:ident)=>{$m! {
 members:$crate::domain::catalog::CatalogMember,member_frames:$crate::domain::catalog::CatalogMemberInvocation,candidates:$crate::domain::catalog::CatalogCandidate,exposures:$crate::domain::catalog::CatalogExposure,paths:$crate::domain::catalog::CatalogPath,aliases:$crate::domain::catalog::CatalogAlias,
 public_exposures:$crate::domain::normalized::entities::PublicExposure,entity_candidates:$crate::domain::normalized::entities::SymbolEntityCandidate,refs:$crate::domain::normalized::entities::EntityRef,callables:$crate::domain::normalized::entities::CallableEntity,classes:$crate::domain::normalized::entities::ClassEntity,
 core_invocations:$crate::domain::analysis::catalog_core::Invocation,modules:$crate::domain::source::Module,artifacts:$crate::domain::source::SourceArtifact,occurrences:$crate::domain::source::Occurrence,chunks:$crate::domain::artifact::ArtifactChunk,
 declarations:$crate::domain::syntax::DeclarationObservation,placements:$crate::domain::syntax::SyntaxPlacement,details:$crate::domain::syntax::SyntaxDetailObservation,detail_values:$crate::domain::syntax::SyntaxDetail,literals:$crate::domain::value::Literal,
 components:$crate::domain::documents::DocumentComponentObservation,component_attributes:$crate::domain::documents::DocumentAttributeObservation,component_values:$crate::domain::documents::DocumentAttributeValue,
 options:$crate::domain::catalog::CatalogOption,option_subjects:$crate::domain::catalog::CatalogOptionSubject,option_slots:$crate::domain::normalized::callables::SignatureSlot,option_variants:$crate::domain::normalized::callables::SignatureVariant,signature_parameters:$crate::domain::calls::SignatureParameter,parameter_shapes:$crate::domain::calls::ParameterShape,parameter_links:$crate::domain::normalized::entities::ParameterEntityLink,parameter_signatures:$crate::domain::calls::Signature,
 document_associations:$crate::domain::catalog::evidence::DocumentAssociation,mention_candidates:$crate::domain::normalized::links::MentionEntityCandidate,mention_assessments:$crate::domain::normalized::links::MentionEntityAssessment,mentions:$crate::domain::documents::DocumentMentionObservation,passages:$crate::domain::documents::PassageObservation,nodes:$crate::domain::documents::DocumentNode,canonical_evidence:$crate::domain::assertion::Evidence,
 native:$crate::domain::analysis::native::NativeAssertionPremise,native_qualifications:$crate::domain::analysis::native::NativeQualification,qualifications:$crate::domain::assertion::AssertionQualification,
}};}
macro_rules! outputs {($m:ident)=>{$m! {conclusions:DocumentaryConclusion,boundaries:DocumentaryBoundary,component_boundaries:crate::domain::synthesis::documentary_templates::ComponentBoundary,sources:DocumentarySource,prose_sources:ProseSource,slices:ProseSlice,qualifications:AssertionQualification,}};}
macro_rules! data {($($f:ident:$ty:ty,)*)=>{
 pub struct Data{$(pub $f:Rows<$ty>,)*}
 impl Data {pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}} pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)} pub fn validation_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]} pub fn facts_inputs()->Vec<ValidationInput>{crate::domain::normalized::facts_inputs(Self::validation_inputs())} pub fn stage_inputs()->Vec<stages::RelationUse>{{vec![$(stages::RelationUse::stored::<$ty>()),*]}}}
};}
crate::synthesis_documentary_inputs!(data);
macro_rules! output {($($f:ident:$ty:ty,)*)=>{
 pub struct Output{$(pub $f:Rows<$ty>,)*}
 impl Output {pub fn new(b:&ResourceBudget)->Self{Self{$($f:Rows::new(b),)*}} pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{$(if n==<$ty>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(false)} pub fn validation_inputs()->Vec<ValidationInput>{vec![$(ValidationInput::of::<$ty>(&["id"]),)*]} pub fn matches(&self,other:&Self)->Result<(),ModelError>{if !self.component_boundaries.same(&other.component_boundaries)||!self.conclusions.same(&other.conclusions)||!self.boundaries.same(&other.boundaries){return Err(invalid("documentary conclusion/boundary closure differs"));}if !self.sources.same(&other.sources)||!self.prose_sources.same(&other.prose_sources)||!self.slices.same(&other.slices){return Err(invalid("documentary slice closure differs"));}for row in other.qualifications.iter(){if self.qualifications.get(row.id())!=Some(row){return Err(invalid("documentary qualification differs"));}}Ok(())}}
};}
outputs!(output);
fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("documentary input absent: {}", R::NAME)))
}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<DocumentaryConclusion>(),
        Relation::of::<DocumentaryBoundary>(),
        Relation::of::<super::documentary_templates::ComponentBoundary>(),
        Relation::of::<ProseSlice>(),
        Relation::of::<ProseSource>(),
        Relation::of::<DocumentarySource>(),
    ]
}
pub(super) fn entity(d: &Data, c: &CatalogCandidate) -> Result<Option<Id<EntityRef>>, ModelError> {
    if let Some(p) = c.path {
        return Ok(Some(need(&d.paths, p)?.entity));
    }
    if let Some(a) = c.alias {
        return Ok(Some(need(&d.aliases, a)?.entity));
    }
    c.entity
        .map(|e| need(&d.entity_candidates, e).map(|r| r.entity))
        .transpose()
}
fn source_declaration(d: &Data, id: Id<EntityRef>) -> Result<Option<Id<Occurrence>>, ModelError> {
    Ok(match need(&d.refs, id)? {
        EntityRef::Callable { callable } => match need(&d.callables, *callable)? {
            CallableEntity::Source { declaration, .. } => Some(*declaration),
            _ => None,
        },
        EntityRef::Class { class } => match need(&d.classes, *class)? {
            ClassEntity::Source { declaration } => Some(*declaration),
            _ => None,
        },
        _ => None,
    })
}
fn native(
    d: &Data,
    predicate: impl Fn(&NativeAssertionPremise) -> bool,
    q: Id<AssertionQualification>,
) -> Result<Option<&NativeAssertionPremise>, ModelError> {
    let mut found = None;
    for p in d.native.iter().filter(|p| predicate(p)) {
        let Some(n) = d.native_qualifications.get(Id::of(
            &crate::domain::analysis::native::NativeQualificationKey { premise: p.id() },
        )) else {
            continue;
        };
        if n.qualification != q || n.family != p.family() || n.fidelity == Fidelity::DisplayOnly {
            continue;
        }
        if found.is_none_or(|old: &NativeAssertionPremise| p.id() < old.id()) {
            found = Some(p);
        }
    }
    Ok(found)
}
pub struct Bytes {
    pub value: String,
    pub(super) _reservation: Box<dyn Reservation>,
}
fn read(d: &Data, occurrence: &Occurrence, b: &ResourceBudget) -> Result<Bytes, ModelError> {
    read_range(d, occurrence.source, occurrence.start, occurrence.end, b)
}
pub(super) fn read_range(
    d: &Data,
    source: Id<SourceArtifact>,
    start: i64,
    end: i64,
    b: &ResourceBudget,
) -> Result<Bytes, ModelError> {
    let artifact = need(&d.artifacts, source)?;
    if start < 0 || end < start || end > artifact.byte_len {
        return Err(invalid("documentary occurrence lies outside artifact"));
    }
    let len = (end - start) as usize;
    let reservation = b.reserve(
        "synthesis-original-prose",
        len.checked_mul(2)
            .and_then(|n| n.checked_add(size_of::<Bytes>()))
            .ok_or_else(|| invalid("documentary source allowance overflow"))?,
    )?;
    let mut bytes = Vec::with_capacity(len);
    let mut offset = start as usize;
    while offset < end as usize {
        let chunk = need(
            &d.chunks,
            Id::of(&ArtifactChunkKey {
                artifact: artifact.id(),
                ordinal: (offset / ARTIFACT_CHUNK_BYTES) as i64,
            }),
        )?;
        let start = offset % ARTIFACT_CHUNK_BYTES;
        let count = (end as usize - offset).min(ARTIFACT_CHUNK_BYTES - start);
        bytes.extend_from_slice(
            chunk
                .body
                .0
                .get(start..start + count)
                .ok_or_else(|| invalid("documentary source chunk missing"))?,
        );
        offset += count;
    }
    Ok(Bytes {
        value: String::from_utf8(bytes).map_err(|_| invalid("documentary source is not UTF-8"))?,
        _reservation: reservation,
    })
}
/// A single literal whose authored interior is the stored interpreted string has exact byte mapping.
/// This operation supplies byte-exact mapping only; interpreted literals have a separate anchor.
fn literal_interior(original: &str) -> Option<(usize, &str)> {
    let prefix = original
        .bytes()
        .take_while(|c| c.is_ascii_alphabetic())
        .count();
    let flags = &original[..prefix];
    if !matches!(flags, "" | "r" | "R" | "u" | "U") {
        return None;
    }
    let rest = &original[prefix..];
    let quote = if rest.starts_with("\"\"\"") {
        "\"\"\""
    } else if rest.starts_with("'''") {
        "'''"
    } else if rest.starts_with('"') {
        "\""
    } else if rest.starts_with('\'') {
        "'"
    } else {
        return None;
    };
    let body = rest.strip_prefix(quote)?.strip_suffix(quote)?;
    if body.contains(quote) {
        return None;
    }
    Some((prefix + quote.len(), body))
}
pub fn literal_content<'a>(original: &'a str, interpreted: &str) -> Option<(usize, &'a str)> {
    literal_interior(original).filter(|(_, body)| *body == interpreted)
}
/// Byte offsets into the original prose. A same-length line-break view avoids splitting soft wraps.
pub fn first_sentence(
    prose: &str,
    b: &ResourceBudget,
) -> Result<Option<(usize, usize)>, ModelError> {
    let trimmed = prose.trim_start();
    let start = prose.len() - trimmed.len();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let mut end = prose.len();
    let mut offset = start;
    for (index, line) in trimmed.split_inclusive('\n').enumerate() {
        let t = line.trim();
        if index > 0
            && (t.is_empty()
                || t.ends_with(':')
                || t.starts_with(':')
                || t.starts_with(">>>")
                || t.chars().all(|c| c == '-' || c == '='))
        {
            end = offset;
            break;
        }
        offset += line.len();
    }
    let Some(segment) = prose.get(start..end) else {
        return Err(invalid("documentary paragraph offsets differ"));
    };
    let _reservation = b.reserve("synthesis-sentence-boundary", segment.len())?;
    let view = segment.replace(['\r', '\n'], " ");
    let Some((i, s)) = view.split_sentence_bound_indices().next() else {
        return Ok(None);
    };
    let s = s.trim_end();
    if s.is_empty() {
        return Ok(None);
    }
    Ok(Some((start + i, start + i + s.len())))
}
fn boundary(
    o: &mut Output,
    m: Id<CatalogMemberInvocation>,
    c: Option<Id<CatalogCandidate>>,
    reason: DocumentaryBoundaryReason,
) -> Result<(), ModelError> {
    o.boundaries.insert(DocumentaryBoundary {
        member: m,
        candidate: c,
        association: None,
        reason,
    })?;
    Ok(())
}
pub fn build(d: &Data, b: &ResourceBudget) -> Result<Output, ModelError> {
    let _admission = b.reserve("synthesis-documentary-build", 512)?;
    let mut out = Output::new(b);
    for frame in d.member_frames.iter() {
        let member = need(&d.members, frame.member)?;
        let invocation = need(&d.core_invocations, frame.invocation)?;
        if member.input != invocation.input {
            return Err(invalid("documentary member crosses input"));
        }
        let mut candidates = 0;
        for candidate in d.candidates.iter() {
            let link = need(&d.exposures, candidate.exposure)?;
            if link.member != member.id() {
                continue;
            }
            let public = need(&d.public_exposures, link.exposure)?;
            if public.context != invocation.context {
                continue;
            }
            candidates += 1;
            let Some(subject) = entity(d, candidate)? else {
                boundary(
                    &mut out,
                    frame.id(),
                    Some(candidate.id()),
                    DocumentaryBoundaryReason::NoSourceDeclaration,
                )?;
                continue;
            };
            let Some(declaration) = source_declaration(d, subject)? else {
                boundary(
                    &mut out,
                    frame.id(),
                    Some(candidate.id()),
                    DocumentaryBoundaryReason::NoSourceDeclaration,
                )?;
                continue;
            };
            let source = need(&d.occurrences, declaration)?.source;
            if need(&d.artifacts, source)?.input != invocation.input {
                return Err(invalid("documentary source crosses input"));
            }
            let mut declarations = 0;
            for row in d
                .declarations
                .iter()
                .filter(|r| r.declaration == declaration)
            {
                let q = need(&d.qualifications, row.qualification)?;
                if q.context != invocation.context {
                    continue;
                }
                declarations += 1;
                let Some(docstring) = row.docstring else {
                    boundary(
                        &mut out,
                        frame.id(),
                        Some(candidate.id()),
                        DocumentaryBoundaryReason::NoDocstring,
                    )?;
                    continue;
                };
                let Some(declaration_premise) = native(
                    d,
                    |p| matches!(p,NativeAssertionPremise::DeclarationObservation{assertion,..}if *assertion==row.id()),
                    row.qualification,
                )?
                else {
                    boundary(
                        &mut out,
                        frame.id(),
                        Some(candidate.id()),
                        DocumentaryBoundaryReason::NativeEvidenceUnavailable,
                    )?;
                    continue;
                };
                let statement = need(&d.occurrences, docstring)?;
                if statement.source != source || statement.syntax_kind != SyntaxKind::StmtExpr {
                    return Err(invalid("documentary statement ownership differs"));
                }
                let mut children = 0;
                for placement in d.placements.iter().filter(|p| {
                    p.parent == Some(docstring)
                        && p.field == crate::domain::lexical::SyntaxField::Value
                        && p.ordinal == 0
                        && p.qualification == row.qualification
                }) {
                    let occurrence = need(&d.occurrences, placement.occurrence)?;
                    if occurrence.source != source
                        || occurrence.syntax_kind != SyntaxKind::ExprStringLiteral
                    {
                        continue;
                    }
                    children += 1;
                    if children > 1 {
                        return Err(invalid("ambiguous documentary literal placement"));
                    }
                    let Some(placement_premise) = native(
                        d,
                        |p| matches!(p,NativeAssertionPremise::SyntaxPlacement{assertion,..}if *assertion==placement.id()),
                        placement.qualification,
                    )?
                    else {
                        boundary(
                            &mut out,
                            frame.id(),
                            Some(candidate.id()),
                            DocumentaryBoundaryReason::NativeEvidenceUnavailable,
                        )?;
                        continue;
                    };
                    let mut details = 0;
                    for detail in d.details.iter().filter(|r| {
                        r.occurrence == occurrence.id() && r.qualification == row.qualification
                    }) {
                        let SyntaxDetail::Literal { literal } =
                            need(&d.detail_values, detail.detail)?
                        else {
                            continue;
                        };
                        let Literal::String { value } = need(&d.literals, *literal)? else {
                            continue;
                        };
                        details += 1;
                        if details > 1 {
                            return Err(invalid("ambiguous documentary string detail"));
                        }
                        let Some(detail_premise) = native(
                            d,
                            |p| matches!(p,NativeAssertionPremise::SyntaxDetailObservation{assertion,..}if *assertion==detail.id()),
                            detail.qualification,
                        )?
                        else {
                            boundary(
                                &mut out,
                                frame.id(),
                                Some(candidate.id()),
                                DocumentaryBoundaryReason::NativeEvidenceUnavailable,
                            )?;
                            continue;
                        };
                        let bytes = read(d, occurrence, b)?;
                        let (body_offset, prose, prose_source) =
                            match literal_content(&bytes.value, value.as_str()) {
                                Some((offset, prose)) => (
                                    offset,
                                    prose,
                                    ProseSource::Occurrence {
                                        occurrence: occurrence.id(),
                                    },
                                ),
                                None => {
                                    boundary(
                                        &mut out,
                                        frame.id(),
                                        Some(candidate.id()),
                                        DocumentaryBoundaryReason::UnsupportedLiteralMapping,
                                    )?;
                                    let prefix = bytes
                                        .value
                                        .bytes()
                                        .take_while(|c| c.is_ascii_alphabetic())
                                        .count();
                                    if !matches!(&bytes.value[..prefix], "" | "r" | "R" | "u" | "U")
                                        || literal_interior(&bytes.value)
                                            .is_some_and(|(_, body)| !body.contains('\\'))
                                    {
                                        continue;
                                    } // Native String detail owns interpretation; whole occurrence owns the original anchor.
                                    (
                                        0,
                                        value.as_str(),
                                        ProseSource::Literal {
                                            occurrence: occurrence.id(),
                                            literal: *literal,
                                        },
                                    )
                                }
                            };
                        let Some((start, end)) = first_sentence(prose, b)? else {
                            boundary(
                                &mut out,
                                frame.id(),
                                Some(candidate.id()),
                                DocumentaryBoundaryReason::EmptySummary,
                            )?;
                            continue;
                        };
                        let prose_source = out.prose_sources.insert(prose_source)?;
                        let prose_id = out.slices.insert(ProseSlice {
                            source: prose_source,
                            start: body_offset as i64,
                            end: (body_offset + prose.len()) as i64,
                        })?;
                        let excerpt = out.slices.insert(ProseSlice {
                            source: prose_source,
                            start: (body_offset + start) as i64,
                            end: (body_offset + end) as i64,
                        })?;
                        let qualification = association_qualification(invocation);
                        out.qualifications.insert(qualification.clone())?;
                        let proof = out.sources.insert(DocumentarySource::Literal {
                            member: frame.id(),
                            candidate: candidate.id(),
                            declaration: declaration_premise.id(),
                            placement: placement_premise.id(),
                            detail: detail_premise.id(),
                            subject,
                            docstring,
                            literal: *literal,
                            source_input: invocation.input,
                            source_qualification: q.id(),
                        })?;
                        out.conclusions.insert(DocumentaryConclusion {
                            source: proof,
                            prose: prose_id,
                            excerpt,
                            excerpt_digest: ContentHash::of(&prose.as_bytes()[start..end]),
                            qualification: qualification.id(),
                            status: EvidenceStatus::Documented,
                        })?;
                    }
                    if details == 0 {
                        boundary(
                            &mut out,
                            frame.id(),
                            Some(candidate.id()),
                            DocumentaryBoundaryReason::NativeEvidenceUnavailable,
                        )?;
                    }
                }
                if children == 0 {
                    boundary(
                        &mut out,
                        frame.id(),
                        Some(candidate.id()),
                        DocumentaryBoundaryReason::NativeEvidenceUnavailable,
                    )?;
                }
            }
            if declarations == 0 {
                boundary(
                    &mut out,
                    frame.id(),
                    Some(candidate.id()),
                    DocumentaryBoundaryReason::NoSourceDeclaration,
                )?;
            }
        }
        if candidates == 0 {
            boundary(
                &mut out,
                frame.id(),
                None,
                DocumentaryBoundaryReason::NoSourceDeclaration,
            )?;
        }
    }
    passage_outcomes(d, &mut out, b)?;
    super::documentary_templates::build(d, &mut out, b)?;
    Ok(out)
}
/// Replay inputs retain the completed vocabulary owner independently of current outputs.
pub(super) fn replay_inputs(inputs: Vec<ValidationInput>, view: stages::PublicationBoundary) -> Vec<ValidationInput> {
    inputs.into_iter().map(|input| if stages::is_vocabulary(input.name()) { input.at_epoch(view) } else { input }).collect()
}
#[cfg(test)]
pub(super) fn replay_input<R: Record>(view: stages::PublicationBoundary) -> ValidationInput {
    let input = ValidationInput::of::<R>(&["id"]);
    if stages::is_vocabulary(input.name()) { input.at_epoch(view) } else { input }
}
pub(super) fn replay_visit(
    input: &ValidationInput, view: Option<stages::PublicationBoundary>,
    visit: impl FnOnce(&str) -> Result<bool, ModelError>,
) -> Result<bool, ModelError> {
    let selected = if stages::is_vocabulary(input.name()) { input.prefix() == view } else { input.prefix().is_none() };
    if selected { visit(input.name()) } else { Ok(false) }
}
pub(super) fn replay_selector(input: &ValidationInput) -> Result<(), ModelError> {
    if input.prefix().is_some() && !stages::is_vocabulary(input.name()) {
        return Err(ModelError::Invalid("S0 replay prefix applies only to vocabulary".into()));
    }
    Ok(())
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = replay_inputs(Data::validation_inputs(), stages::PublicationBoundary::Facts);
    inputs.extend(Output::validation_inputs());
    inputs.sort_by_key(|input| (input.name(), input.prefix()));
    inputs.dedup_by_key(|input| (input.name(), input.prefix()));
    vec![Invariant {
        revision: 2,
        name: "synthesis_documentary_replay",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                budget: b.clone(),
                data: Data::new(b),
                output: Output::new(b),
            })
        }),
    }]
}
struct Check {
    budget: ResourceBudget,
    data: Data,
    output: Output,
}
impl InvariantCheck for Check {
    fn visit(&mut self, _name: &str, _batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Err(ModelError::Invalid("S0 replay requires an explicit completed-input selector".into()))
    }
    fn visit_input(&mut self, input: &ValidationInput, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        replay_selector(input)?;
        let native = replay_visit(input, Some(stages::PublicationBoundary::Facts), |name| self.data.visit(name, b))?;
        let output = replay_visit(input, None, |name| self.output.visit(name, b))?;
        if !native && !output {
            return Err(invalid("undeclared documentary invariant input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        let expected = build(&self.data, &self.budget)?;
        self.output.matches(&expected)
    }
}

/// Resolve an original anchor mechanically; a derived slice never changes the earlier source.
pub fn original(
    d: &Data,
    source: &ProseSource,
) -> Result<(Id<SourceArtifact>, i64, i64), ModelError> {
    match source {
        ProseSource::Occurrence { occurrence } | ProseSource::Literal { occurrence, .. } => {
            let row = need(&d.occurrences, *occurrence)?;
            Ok((row.source, row.start, row.end))
        }
        ProseSource::Span { span } => match need(&d.canonical_evidence, span.id())? {
            assertion::Evidence::SourceSpan { source, start, end } => Ok((*source, *start, *end)),
            _ => Err(invalid("documentary anchor is not an earlier source span")),
        },
    }
}
pub fn read_slice(
    d: &Data,
    rows: &Output,
    slice: &ProseSlice,
    b: &ResourceBudget,
) -> Result<Bytes, ModelError> {
    let source = need(&rows.prose_sources, slice.source)?;
    if let ProseSource::Literal {
        occurrence,
        literal,
    } = source
    {
        let original = need(&d.occurrences, *occurrence)?;
        let _original = read(d, original, b)?;
        let Literal::String { value } = need(&d.literals, *literal)? else {
            return Err(invalid(
                "interpreted documentary anchor is not a native string",
            ));
        };
        let start = usize::try_from(slice.start).map_err(ModelError::codec)?;
        let end = usize::try_from(slice.end).map_err(ModelError::codec)?;
        let text = value
            .get(start..end)
            .ok_or_else(|| invalid("interpreted documentary slice exceeds native literal"))?;
        let reservation = b.reserve(
            "native-literal-prose-slice",
            text.len() + size_of::<Bytes>(),
        )?;
        return Ok(Bytes {
            value: text.to_owned(),
            _reservation: reservation,
        });
    }
    let (artifact, start, end) = original(d, source)?;
    if slice.start < 0 || slice.end < slice.start || slice.end > end - start {
        return Err(invalid("documentary slice exceeds original anchor"));
    }
    read_range(
        d,
        artifact,
        start
            .checked_add(slice.start)
            .ok_or_else(|| invalid("documentary slice start overflow"))?,
        start
            .checked_add(slice.end)
            .ok_or_else(|| invalid("documentary slice end overflow"))?,
        b,
    )
}
/// Paragraph boundaries are source-byte operations, not declarations or symbol resolution.
pub fn mention_sentence(
    text: &str,
    mention: (usize, usize),
    b: &ResourceBudget,
) -> Result<Option<(usize, usize)>, ModelError> {
    if mention.0 > mention.1
        || mention.1 > text.len()
        || !text.is_char_boundary(mention.0)
        || !text.is_char_boundary(mention.1)
    {
        return Err(invalid("document mention exceeds original passage"));
    }
    let mut offset = 0;
    let mut fenced = false;
    let mut open: Option<(usize, usize)> = None;
    let finish = |range: Option<(usize, usize)>| -> Result<Option<(usize, usize)>, ModelError> {
        let Some((start, end)) = range else {
            return Ok(None);
        };
        if mention.0 < start || mention.1 > end {
            return Ok(None);
        }
        let Some((a, z)) = first_sentence(&text[start..end], b)? else {
            return Ok(None);
        };
        let (a, z) = (start + a, start + z);
        Ok((mention.0 >= a && mention.1 <= z).then_some((a, z)))
    };
    for line in text.split_inclusive('\n') {
        let at = offset;
        offset += line.len();
        let body = line.trim();
        let fence = body.starts_with("```") || body.starts_with("~~~");
        let skip = fence
            || fenced
            || body.is_empty()
            || body.starts_with(['#', '<', '>', '|'])
            || body.starts_with(":::")
            || body.starts_with("import ")
            || body.starts_with("export ")
            || body.starts_with("---");
        let trim = line.len() - line.trim_start().len();
        let trimmed = &line[trim..];
        let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
        let marker = if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
        {
            Some(trim + 2)
        } else if digits > 0 && trimmed[digits..].starts_with(". ") {
            Some(trim + digits + 2)
        } else {
            None
        };
        if (skip || marker.is_some())
            && let Some(result) = finish(open.take())?
        {
            return Ok(Some(result));
        }
        if fence {
            fenced = !fenced;
            continue;
        }
        if skip {
            continue;
        }
        let end = at + line.trim_end().len();
        if let Some(marker) = marker {
            open = Some((at + marker, end));
        } else if let Some((_, end0)) = &mut open {
            *end0 = end;
        } else {
            open = Some((at + trim, end));
        }
    }
    finish(open)
}
/// Static association: source qualifications remain on their exact earlier native premises.
fn association_qualification(i: &analysis::catalog_core::Invocation) -> AssertionQualification {
    AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: i.context,
        scope: CoverageScope::Input { input: i.input }.id(),
        condition: conditions::Diagram::always().id(),
        modality: Modality::Candidate,
        approximation: Approximation::Over,
    }
}
fn passage_outcomes(d: &Data, out: &mut Output, b: &ResourceBudget) -> Result<(), ModelError> {
    for frame in d.member_frames.iter() {
        let member = need(&d.members, frame.member)?;
        let invocation = need(&d.core_invocations, frame.invocation)?;
        for association in d
            .document_associations
            .iter()
            .filter(|r| r.member == member.id())
        {
            let candidate = need(&d.mention_candidates, association.candidate)?;
            let exposure = need(&d.public_exposures, candidate.exposure)?;
            if exposure.context != invocation.context {
                out.boundaries.insert(DocumentaryBoundary {
                    member: frame.id(),
                    candidate: None,
                    association: Some(association.id()),
                    reason: DocumentaryBoundaryReason::ForeignContext,
                })?;
                continue;
            }
            if !d
                .exposures
                .iter()
                .any(|l| l.member == member.id() && l.exposure == exposure.id())
            {
                return Err(invalid(
                    "documentary association does not name the exact C0 exposure",
                ));
            }
            let assessment = need(&d.mention_assessments, candidate.assessment)?;
            let mention = need(&d.mentions, assessment.observation)?;
            let q = need(&d.qualifications, mention.qualification)?;
            if q.context != invocation.context {
                out.boundaries.insert(DocumentaryBoundary {
                    member: frame.id(),
                    candidate: None,
                    association: Some(association.id()),
                    reason: DocumentaryBoundaryReason::ForeignContext,
                })?;
                continue;
            }
            let mut passages = 0;
            for passage in d.passages.iter().filter(|p| p.passage == mention.passage) {
                let pq = need(&d.qualifications, passage.qualification)?;
                if pq.context != invocation.context {
                    out.boundaries.insert(DocumentaryBoundary {
                        member: frame.id(),
                        candidate: None,
                        association: Some(association.id()),
                        reason: DocumentaryBoundaryReason::ForeignContext,
                    })?;
                    continue;
                }
                passages += 1;
                if pq.scope != q.scope || pq.condition != q.condition {
                    return Err(invalid(
                        "documentary passage and mention change source frame",
                    ));
                }
                let prose_source = ProseSource::Span {
                    span: need(&d.nodes, passage.passage.id())?.span(),
                };
                let (artifact, start, end) = original(d, &prose_source)?;
                let captured = need(&d.artifacts, artifact)?;
                let name = captured
                    .path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&captured.path)
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if name.contains("changelog") || name == "whats-new" || name == "changes" {
                    out.boundaries.insert(DocumentaryBoundary {
                        member: frame.id(),
                        candidate: None,
                        association: Some(association.id()),
                        reason: DocumentaryBoundaryReason::ExcludedChangelog,
                    })?;
                    continue;
                }
                let mention_source = ProseSource::Span {
                    span: need(&d.nodes, mention.mention.id())?.span(),
                };
                let (ma, ms, me) = original(d, &mention_source)?;
                if ma != artifact || ms < start || me > end {
                    return Err(invalid(
                        "documentary mention lies outside exact original passage",
                    ));
                }
                let Some(native_passage) = native(
                    d,
                    |p| matches!(p,NativeAssertionPremise::PassageObservation{assertion,..}if *assertion==passage.id()),
                    passage.qualification,
                )?
                else {
                    out.boundaries.insert(DocumentaryBoundary {
                        member: frame.id(),
                        candidate: None,
                        association: Some(association.id()),
                        reason: DocumentaryBoundaryReason::NativeEvidenceUnavailable,
                    })?;
                    continue;
                };
                let Some(native_mention) = native(
                    d,
                    |p| matches!(p,NativeAssertionPremise::DocumentMentionObservation{assertion,..}if *assertion==mention.id()),
                    mention.qualification,
                )?
                else {
                    out.boundaries.insert(DocumentaryBoundary {
                        member: frame.id(),
                        candidate: None,
                        association: Some(association.id()),
                        reason: DocumentaryBoundaryReason::NativeEvidenceUnavailable,
                    })?;
                    continue;
                };
                let bytes = read_range(d, artifact, start, end, b)?;
                let Some((first, last)) = mention_sentence(
                    &bytes.value,
                    ((ms - start) as usize, (me - start) as usize),
                    b,
                )?
                else {
                    out.boundaries.insert(DocumentaryBoundary {
                        member: frame.id(),
                        candidate: None,
                        association: Some(association.id()),
                        reason: DocumentaryBoundaryReason::NoLeadMention,
                    })?;
                    continue;
                };
                let base = out.prose_sources.insert(prose_source)?;
                let prose = out.slices.insert(ProseSlice {
                    source: base,
                    start: 0,
                    end: end - start,
                })?;
                let excerpt = out.slices.insert(ProseSlice {
                    source: base,
                    start: first as i64,
                    end: last as i64,
                })?;
                let source = out.sources.insert(DocumentarySource::Passage {
                    member: frame.id(),
                    association: association.id(),
                    passage: native_passage.id(),
                    mention: native_mention.id(),
                    source_input: captured.input,
                    source_qualification: q.id(),
                })?;
                let qualification = association_qualification(invocation);
                out.qualifications.insert(qualification.clone())?;
                out.conclusions.insert(DocumentaryConclusion {
                    source,
                    prose,
                    excerpt,
                    excerpt_digest: ContentHash::of(&bytes.value.as_bytes()[first..last]),
                    qualification: qualification.id(),
                    status: analysis::policy::derive_status(&[
                        (
                            analysis::policy::SupportRole::Support,
                            need(
                                &d.native_qualifications,
                                Id::of(&analysis::native::NativeQualificationKey {
                                    premise: native_passage.id(),
                                }),
                            )?
                            .status,
                        ),
                        (
                            analysis::policy::SupportRole::Support,
                            need(
                                &d.native_qualifications,
                                Id::of(&analysis::native::NativeQualificationKey {
                                    premise: native_mention.id(),
                                }),
                            )?
                            .status,
                        ),
                    ]),
                })?;
            }
            if passages == 0 {
                out.boundaries.insert(DocumentaryBoundary {
                    member: frame.id(),
                    candidate: None,
                    association: Some(association.id()),
                    reason: DocumentaryBoundaryReason::NativeEvidenceUnavailable,
                })?;
            }
        }
    }
    Ok(())
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["synthesis_documentary_replay"]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::domain::{analysis::native::NativeQualification, artifact::ArtifactChunk};
    #[test]
    fn replay_inventories_preserve_native_retained_and_output_qualification_views() {
        use stages::PublicationBoundary::{Facts, Analytic};
        let controls = [
            (invariants(), vec![None, Some(Facts)]),
            (super::super::patterns::invariants(), vec![None, Some(Facts)]),
            (super::super::seeds::invariants(), vec![None, Some(Facts)]),
            (super::super::summary::invariants(), vec![Some(Facts)]),
            (super::super::observations::invariants(), vec![None, Some(Facts), Some(Analytic)]),
            (super::super::assertions::invariants(), vec![None, Some(Facts), Some(Analytic)]),
            (super::super::briefs::invariants(), vec![None, Some(Facts), Some(Analytic)]),
        ];
        for (controls, expected) in controls {
            for control in controls {
                assert_eq!(control.revision, 2);
                let views = control.inputs.iter().filter(|input| input.name() == AssertionQualification::NAME).map(ValidationInput::prefix).collect::<std::collections::BTreeSet<_>>();
                assert_eq!(views, expected.iter().copied().collect::<std::collections::BTreeSet<_>>(), "{}", control.name);
            }
        }
    }
    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }
    pub(in crate::domain::synthesis) fn pair(
        d: &mut Data,
        p: NativeAssertionPremise,
        q: &AssertionQualification,
    ) {
        let n = NativeQualification {
            premise: p.id(),
            qualification: q.id(),
            family: p.family(),
            fidelity: Fidelity::NativeStructural,
            status: analysis::policy::native_status(p.family(), Fidelity::NativeStructural),
        };
        d.native.insert(p).unwrap();
        d.native_qualifications.insert(n).unwrap();
    }
    pub(crate) fn fixture(
        original: &str,
        interpreted: &str,
    ) -> (ResourceBudget, Data, Id<CatalogMemberInvocation>) {
        let b = ResourceBudget::fixed(32 << 20).unwrap();
        let mut d = Data::new(&b);
        let source =
            SourceArtifact::from_bytes(id(1), "api.py".into(), original.as_bytes()).unwrap();
        for c in ArtifactChunk::split(&source, original.as_bytes()).unwrap() {
            d.chunks.insert(c).unwrap();
        }
        d.artifacts.insert(source.clone()).unwrap();
        let module = d
            .modules
            .insert(Module {
                source: source.id(),
                qualified_name: "pkg.api".into(),
            })
            .unwrap();
        let q = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: id(2),
            scope: CoverageScope::Artifact {
                artifact: source.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        d.qualifications.insert(q.clone()).unwrap();
        let declaration = d
            .occurrences
            .insert(Occurrence {
                source: source.id(),
                start: 0,
                end: original.len() as i64,
                syntax_kind: SyntaxKind::StmtFunctionDef,
                role: OccurrenceRole::Syntax,
                structural_path: vec![],
            })
            .unwrap();
        let docstring = d
            .occurrences
            .insert(Occurrence {
                source: source.id(),
                start: 0,
                end: original.len() as i64,
                syntax_kind: SyntaxKind::StmtExpr,
                role: OccurrenceRole::Syntax,
                structural_path: vec![0],
            })
            .unwrap();
        let literal_occurrence = d
            .occurrences
            .insert(Occurrence {
                source: source.id(),
                start: 0,
                end: original.len() as i64,
                syntax_kind: SyntaxKind::ExprStringLiteral,
                role: OccurrenceRole::Syntax,
                structural_path: vec![0, 0],
            })
            .unwrap();
        let member = d
            .members
            .insert(CatalogMember {
                input: source.input,
                access: module,
                path: vec!["run".into()],
                name: "run".into(),
            })
            .unwrap();
        let invocation =
            analysis::catalog_core::Invocation::new(source.input, q.context, id(3), None, []).0;
        d.core_invocations.insert(invocation.clone()).unwrap();
        let frame = d
            .member_frames
            .insert(CatalogMemberInvocation {
                member,
                invocation: invocation.id(),
            })
            .unwrap();
        let exposure = d
            .public_exposures
            .insert(PublicExposure {
                access: module,
                context: q.context,
                observation: id(4),
                origin: id(5),
                enumeration: None,
                publicity: crate::domain::normalized::entities::PublicPathKnowledge::Known,
                status: ResolutionStatus::Unresolved,
                reason: EntityReason::UntracedExposure,
            })
            .unwrap();
        let link = d
            .exposures
            .insert(CatalogExposure { member, exposure })
            .unwrap();
        let callable = d
            .callables
            .insert(CallableEntity::Source {
                declaration,
                kind: CallableKind::Function,
            })
            .unwrap();
        let entity = d.refs.insert(EntityRef::Callable { callable }).unwrap();
        let ec = d
            .entity_candidates
            .insert(SymbolEntityCandidate {
                resolution: id(6),
                entity,
            })
            .unwrap();
        d.candidates
            .insert(CatalogCandidate {
                exposure: link,
                candidate: Some(id(7)),
                entity: Some(ec),
                path: None,
                alias: None,
            })
            .unwrap();
        let row = DeclarationObservation {
            qualification: q.id(),
            declaration,
            name: id(8),
            kind: DeclarationKind::Function,
            parent: None,
            overload: false,
            docstring: Some(docstring),
        };
        d.declarations.insert(row.clone()).unwrap();
        pair(
            &mut d,
            NativeAssertionPremise::DeclarationObservation {
                assertion: row.id(),
                support: id(9),
            },
            &q,
        );
        let placement = SyntaxPlacement {
            qualification: q.id(),
            occurrence: literal_occurrence,
            parent: Some(docstring),
            field: crate::domain::lexical::SyntaxField::Value,
            ordinal: 0,
        };
        d.placements.insert(placement.clone()).unwrap();
        pair(
            &mut d,
            NativeAssertionPremise::SyntaxPlacement {
                assertion: placement.id(),
                support: id(10),
            },
            &q,
        );
        let literal = d
            .literals
            .insert(Literal::String {
                value: interpreted.into(),
            })
            .unwrap();
        let value = d
            .detail_values
            .insert(SyntaxDetail::Literal { literal })
            .unwrap();
        let detail = SyntaxDetailObservation {
            qualification: q.id(),
            occurrence: literal_occurrence,
            ordinal: 0,
            detail: value,
        };
        d.details.insert(detail.clone()).unwrap();
        pair(
            &mut d,
            NativeAssertionPremise::SyntaxDetailObservation {
                assertion: detail.id(),
                support: id(11),
            },
            &q,
        );
        (b, d, frame)
    }
    pub(in crate::domain::synthesis) fn replay(
        d: &Data,
        o: &Output,
        b: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let mut c = (invariants().remove(0).create)(b);
        macro_rules! input{($($f:ident:$ty:ty,)*)=>{$(c.visit_input(&replay_input::<$ty>(stages::PublicationBoundary::Facts),&<$ty as Record>::encode(&d.$f.iter().cloned().collect::<Vec<_>>())?)?;)*};}
        crate::synthesis_documentary_inputs!(input);
        macro_rules! output{($($f:ident:$ty:ty,)*)=>{$(c.visit_input(&ValidationInput::of::<$ty>(&["id"]),&<$ty as Record>::encode(&o.$f.iter().cloned().collect::<Vec<_>>())?)?;)*};}
        outputs!(output);
        c.finish()
    }
    pub(in crate::domain::synthesis) fn passage_fixture(
        cross_input: bool,
        text: &str,
        mention: (usize, usize),
    ) -> (
        ResourceBudget,
        Data,
        Id<CatalogMemberInvocation>,
        Id<SourceArtifact>,
    ) {
        let (b, mut d, frame) = fixture("\"Run.\"", "Run.");
        let input = if cross_input {
            id(70)
        } else {
            d.artifacts.iter().next().unwrap().input
        };
        let artifact =
            SourceArtifact::from_bytes(input, "guide.mdx".into(), text.as_bytes()).unwrap();
        for row in ArtifactChunk::split(&artifact, text.as_bytes()).unwrap() {
            d.chunks.insert(row).unwrap();
        }
        d.artifacts.insert(artifact.clone()).unwrap();
        let context = d.core_invocations.iter().next().unwrap().context;
        let q = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context,
            scope: CoverageScope::Artifact {
                artifact: artifact.id(),
            }
            .id(),
            condition: conditions::Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        d.qualifications.insert(q.clone()).unwrap();
        let span = assertion::Evidence::SourceSpan {
            source: artifact.id(),
            start: 0,
            end: text.len() as i64,
        };
        let span_id = assertion::EvidenceSourceSpanId::of(&span).unwrap();
        d.canonical_evidence.insert(span).unwrap();
        let node = documents::DocumentNode::Passage {
            span: span_id,
            ordinal: 0,
        };
        let passage = documents::DocumentNodePassageId::of(&node).unwrap();
        d.nodes.insert(node).unwrap();
        let span = assertion::Evidence::SourceSpan {
            source: artifact.id(),
            start: mention.0 as i64,
            end: mention.1 as i64,
        };
        let span_id = assertion::EvidenceSourceSpanId::of(&span).unwrap();
        d.canonical_evidence.insert(span).unwrap();
        let node = documents::DocumentNode::Mention { span: span_id };
        let mention_node = documents::DocumentNodeMentionId::of(&node).unwrap();
        d.nodes.insert(node).unwrap();
        let row = documents::PassageObservation {
            qualification: q.id(),
            passage,
            level: 0,
            heading: None,
            heading_path: vec![],
            text: "invented provider display must never become authored evidence".into(),
        };
        d.passages.insert(row.clone()).unwrap();
        pair(
            &mut d,
            NativeAssertionPremise::PassageObservation {
                assertion: row.id(),
                support: id(71),
            },
            &q,
        );
        let row = documents::DocumentMentionObservation {
            qualification: q.id(),
            mention: mention_node,
            passage,
            class: documents::MentionClass::Exact,
            source: documents::MentionSource::InlineCode,
            form: text[mention.0..mention.1].into(),
            access_path: Some("pkg.api.run".into()),
            qualified_name: None,
        };
        d.mentions.insert(row.clone()).unwrap();
        pair(
            &mut d,
            NativeAssertionPremise::DocumentMentionObservation {
                assertion: row.id(),
                support: id(72),
            },
            &q,
        );
        let assessment = d
            .mention_assessments
            .insert(normalized::links::MentionEntityAssessment {
                observation: row.id(),
                status: ResolutionStatus::Unresolved,
                reason: normalized::links::LinkReason::MissingResolution,
            })
            .unwrap();
        let exposure = d.public_exposures.iter().next().unwrap().id();
        let candidate = d
            .mention_candidates
            .insert(normalized::links::MentionEntityCandidate {
                assessment,
                exposure,
            })
            .unwrap();
        let member = d.member_frames.get(frame).unwrap().member;
        d.document_associations
            .insert(evidence::DocumentAssociation {
                member,
                candidate,
                basis: evidence::AssociationBasis::DocumentCandidate,
            })
            .unwrap();
        (b, d, frame, artifact.id())
    }
    #[test]
    fn source_native_prose_and_release_association_are_distinct_for_same_and_cross_input() {
        for cross in [false, true] {
            let text = "`pkg.api.run` starts a session. Then finish.\n\nWarning: preserve the credentials.\n";
            let (b, d, frame, artifact) = passage_fixture(cross, text, (1, 12));
            let out = build(&d, &b).unwrap();
            let c = out
                .conclusions
                .iter()
                .find(|r| {
                    matches!(
                        out.sources.get(r.source),
                        Some(DocumentarySource::Passage { .. })
                    )
                })
                .unwrap();
            let DocumentarySource::Passage {
                member,
                source_input,
                source_qualification,
                ..
            } = out.sources.get(c.source).unwrap()
            else {
                panic!()
            };
            assert_eq!(*member, frame);
            assert_eq!(*source_input, d.artifacts.get(artifact).unwrap().input);
            let nativeq = d.qualifications.get(*source_qualification).unwrap();
            assert_eq!(nativeq.scope, CoverageScope::Artifact { artifact }.id());
            assert_eq!(
                (nativeq.modality, nativeq.approximation),
                (Modality::Definite, Approximation::Exact)
            );
            let q = out.qualifications.get(c.qualification()).unwrap();
            let invocation = d.core_invocations.iter().next().unwrap();
            assert_eq!(
                q.scope,
                CoverageScope::Input {
                    input: invocation.input
                }
                .id()
            );
            assert_eq!(q.context, invocation.context);
            assert_eq!(
                (q.modality, q.approximation),
                (Modality::Candidate, Approximation::Over)
            );
            assert_eq!(q.condition, conditions::Diagram::always().id());
            assert_eq!(c.status(), EvidenceStatus::Documented);
            assert_eq!(
                read_slice(&d, &out, out.slices.get(c.excerpt).unwrap(), &b)
                    .unwrap()
                    .value,
                "`pkg.api.run` starts a session."
            );
            assert!(
                read_slice(&d, &out, out.slices.get(c.prose).unwrap(), &b)
                    .unwrap()
                    .value
                    .contains("preserve the credentials")
            );
            replay(&d, &out, &b).unwrap();
        }
    }
    #[test]
    fn passage_bridge_requires_original_span_exact_exposure_context_and_native_support() {
        let text = "`pkg.api.run` starts.\n";
        for case in 0..4 {
            let (b, mut d, _, _) = passage_fixture(true, text, (1, 12));
            match case {
                0 => {
                    d.exposures = Rows::new(&b);
                    assert!(build(&d, &b).is_err());
                    continue;
                }
                1 => {
                    let mut exposure = d.public_exposures.iter().next().unwrap().clone();
                    exposure.context = id(90);
                    let exposureid = exposure.id();
                    d.public_exposures.insert(exposure).unwrap();
                    let mut candidate = d.mention_candidates.iter().next().unwrap().clone();
                    candidate.exposure = exposureid;
                    d.mention_candidates = Rows::new(&b);
                    let candidate = d.mention_candidates.insert(candidate).unwrap();
                    let mut association = d.document_associations.iter().next().unwrap().clone();
                    association.candidate = candidate;
                    d.document_associations = Rows::new(&b);
                    d.document_associations.insert(association).unwrap();
                }
                2 => {
                    d.native = Rows::new(&b);
                }
                _ => {
                    d.canonical_evidence = Rows::new(&b);
                    assert!(build(&d, &b).is_err());
                    continue;
                }
            }
            let out = build(&d, &b).unwrap();
            assert!(
                !out.sources
                    .iter()
                    .any(|s| matches!(s, DocumentarySource::Passage { .. }))
            );
            assert!(out.boundaries.iter().any(|r| r.association.is_some()
                && r.reason
                    == if case == 1 {
                        DocumentaryBoundaryReason::ForeignContext
                    } else {
                        DocumentaryBoundaryReason::NativeEvidenceUnavailable
                    }));
            replay(&d, &out, &b).unwrap();
        }
    }
    #[test]
    fn paragraph_controls_reject_late_mentions_fences_headers_and_changelog() {
        let b = ResourceBudget::fixed(1 << 20).unwrap();
        for text in [
            "First sentence. Use `pkg.api.run` later.",
            "# `pkg.api.run`\n",
            "```python\n`pkg.api.run`\n```\n",
            "<Thing api=\"pkg.api.run\" />\n",
        ] {
            let start = text.find("pkg.api.run").unwrap();
            assert_eq!(
                mention_sentence(text, (start, start + 11), &b).unwrap(),
                None
            );
        }
        let text = "- `pkg.api.run` starts. More text.\n";
        let start = text.find("pkg.api.run").unwrap();
        assert_eq!(
            mention_sentence(text, (start, start + 11), &b).unwrap(),
            Some((2, 23))
        );
    }
    #[test]
    fn passage_replay_rejects_source_input_qualification_and_coupled_output_removal() {
        let (b, d, _, _) = passage_fixture(true, "`pkg.api.run` starts.\n", (1, 12));
        for case in 0..3 {
            let mut out = build(&d, &b).unwrap();
            if case == 2 {
                out.sources = Rows::new(&b);
                out.conclusions = Rows::new(&b);
                out.slices = Rows::new(&b);
                out.prose_sources = Rows::new(&b);
            } else {
                let mut source = out
                    .sources
                    .iter()
                    .find(|s| matches!(s, DocumentarySource::Passage { .. }))
                    .unwrap()
                    .clone();
                if let DocumentarySource::Passage {
                    source_input,
                    source_qualification,
                    ..
                } = &mut source
                {
                    if case == 0 {
                        *source_input = id(99);
                    } else {
                        *source_qualification = id(98);
                    }
                }
                let original = out
                    .sources
                    .iter()
                    .filter(|s| !matches!(s, DocumentarySource::Passage { .. }))
                    .cloned()
                    .collect::<Vec<_>>();
                out.sources = Rows::new(&b);
                for row in original {
                    out.sources.insert(row).unwrap();
                }
                out.sources.insert(source).unwrap();
            }
            assert!(replay(&d, &out, &b).is_err());
        }
    }
    #[test]
    fn multiple_native_supports_choose_one_exact_canonical_premise() {
        let (b, mut d, _) = fixture("\"Run.\"", "Run.");
        let row = d.declarations.iter().next().unwrap();
        let q = d.qualifications.get(row.qualification).unwrap().clone();
        let assertion = row.id();
        pair(
            &mut d,
            NativeAssertionPremise::DeclarationObservation {
                assertion,
                support: id(92),
            },
            &q,
        );
        let out = build(&d, &b).unwrap();
        assert_eq!(out.conclusions.len(), 1);
        let DocumentarySource::Literal { declaration, .. } = out.sources.iter().next().unwrap()
        else {
            panic!()
        };
        let expected=d.native.iter().filter(|p|matches!(p,NativeAssertionPremise::DeclarationObservation{assertion:a,..}if *a==assertion)).map(Record::id).min().unwrap();
        assert_eq!(*declaration, expected);
        replay(&d, &out, &b).unwrap();
    }
    #[test]
    fn exact_unicode_soft_wrapped_outcome_retains_full_prose_and_candidate() {
        let text = "\"\"\"Run it — fast\n    across contexts. Then stop.\n\nWarnings:\n    This is authored caution.\n\"\"\"";
        let (b, d, frame) = fixture(text, &text[3..text.len() - 3]);
        let out = build(&d, &b).unwrap();
        assert_eq!(out.conclusions.len(), 1);
        assert!(out.boundaries.is_empty());
        let c = out.conclusions.iter().next().unwrap();
        assert_eq!(out.sources.get(c.source).unwrap().member(), frame);
        let ProseSlice { start, end, .. } = out.slices.get(c.excerpt).unwrap();
        assert_eq!(
            &text[*start as usize..*end as usize],
            "Run it — fast\n    across contexts."
        );
        let ProseSlice { start, end, .. } = out.slices.get(c.prose).unwrap();
        assert!(text[*start as usize..*end as usize].contains("authored caution"));
        let q = out.qualifications.get(c.qualification()).unwrap();
        assert_eq!(
            (q.modality, q.approximation),
            (Modality::Candidate, Approximation::Over)
        );
        assert_eq!(c.source_facts().status, EvidenceStatus::Documented);
        replay(&d, &out, &b).unwrap();
    }
    #[test]
    fn unsupported_literal_mapping_is_explicit_and_never_becomes_display_evidence() {
        for (raw, value) in [
            ("f\"Interpolated {x}.\"", "Interpolated {x}."),
            ("b\"Bytes.\"", "Bytes."),
        ] {
            let (b, d, _) = fixture(raw, value);
            let out = build(&d, &b).unwrap();
            assert!(out.conclusions.is_empty());
            assert!(
                out.boundaries
                    .iter()
                    .any(|r| r.reason == DocumentaryBoundaryReason::UnsupportedLiteralMapping)
            );
            replay(&d, &out, &b).unwrap();
        }
        assert_eq!(literal_content("r'''A\\b.'''", "A\\b."), Some((4, "A\\b.")));
    }
    #[test]
    fn native_literal_interpretation_retains_escaped_and_concatenated_summary_with_whole_anchor() {
        for (raw, value) in [
            (
                "\"Escaped\\n    text. Warnings: retain.\"",
                "Escaped\n    text. Warnings: retain.",
            ),
            ("\"First.\" \"Second.\"", "First.Second."),
        ] {
            let (b, d, _) = fixture(raw, value);
            let out = build(&d, &b).unwrap();
            assert_eq!(out.conclusions.len(), 1);
            assert!(
                out.boundaries
                    .iter()
                    .any(|r| r.reason == DocumentaryBoundaryReason::UnsupportedLiteralMapping)
            );
            let c = out.conclusions.iter().next().unwrap();
            let slice = out.slices.get(c.prose).unwrap();
            let source = out.prose_sources.get(slice.source).unwrap();
            assert!(matches!(source, ProseSource::Literal { .. }));
            let (artifact, start, end) = original(&d, source).unwrap();
            assert_eq!((start, end), (0, raw.len() as i64));
            assert_eq!(read_range(&d, artifact, start, end, &b).unwrap().value, raw);
            assert_eq!(read_slice(&d, &out, slice, &b).unwrap().value, value);
            replay(&d, &out, &b).unwrap();
        }
    }
    #[test]
    fn missing_native_support_or_wrong_typed_edge_never_guesses_a_docstring() {
        let (b, mut d, _) = fixture("\"Run.\"", "Run.");
        d.native = Rows::new(&b);
        let out = build(&d, &b).unwrap();
        assert!(out.conclusions.is_empty());
        assert!(
            out.boundaries
                .iter()
                .any(|r| r.reason == DocumentaryBoundaryReason::NativeEvidenceUnavailable)
        );
        let (b, mut d, _) = fixture("\"Run.\"", "Run.");
        let mut p = d.placements.iter().next().unwrap().clone();
        p.field = crate::domain::lexical::SyntaxField::Body;
        d.placements = Rows::new(&b);
        d.placements.insert(p).unwrap();
        assert!(build(&d, &b).unwrap().conclusions.is_empty());
    }
    #[test]
    fn context_and_literal_identity_are_not_spelling_or_equal_span_guesses() {
        let (b, mut d, _) = fixture("\"Run.\"", "Run.");
        let mut p = d.public_exposures.iter().next().unwrap().clone();
        p.context = id(50);
        d.public_exposures = Rows::new(&b);
        d.public_exposures.insert(p).unwrap();
        let mut link = d.exposures.iter().next().unwrap().clone();
        link.exposure = d.public_exposures.iter().next().unwrap().id();
        d.exposures = Rows::new(&b);
        let linkid = d.exposures.insert(link).unwrap();
        let mut c = d.candidates.iter().next().unwrap().clone();
        c.exposure = linkid;
        d.candidates = Rows::new(&b);
        d.candidates.insert(c).unwrap();
        let out = build(&d, &b).unwrap();
        assert!(out.conclusions.is_empty());
        assert!(out.boundaries.iter().any(|r| r.candidate.is_none()));
        let (b, mut d, _) = fixture("\"Run.\"", "Different.");
        assert!(build(&d, &b).unwrap().conclusions.is_empty());
        d.chunks = Rows::new(&b);
        assert!(build(&d, &b).is_err());
    }
    #[test]
    fn replay_refuses_forged_excerpt_qualification_support_or_coupled_deletion() {
        let (b, d, _) = fixture("\"Run it. Then stop.\"", "Run it. Then stop.");
        for case in 0..4 {
            let mut out = build(&d, &b).unwrap();
            let mut row = out.conclusions.iter().next().unwrap().clone();
            match case {
                0 => {
                    row.excerpt_digest = ContentHash::of(b"forged");
                    out.conclusions = Rows::new(&b);
                    out.conclusions.insert(row).unwrap();
                }
                1 => {
                    row.qualification = id(60);
                    out.conclusions = Rows::new(&b);
                    out.conclusions.insert(row).unwrap();
                }
                2 => {
                    row.source = id(61);
                    out.conclusions = Rows::new(&b);
                    out.conclusions.insert(row).unwrap();
                }
                _ => {
                    out.conclusions = Rows::new(&b);
                    out.boundaries = Rows::new(&b);
                    out.slices = Rows::new(&b);
                    out.sources = Rows::new(&b);
                    out.prose_sources = Rows::new(&b);
                    out.qualifications = Rows::new(&b);
                }
            }
            assert!(replay(&d, &out, &b).is_err());
        }
    }
    #[test]
    fn first_paragraph_section_boundaries_empty_and_budget_refusal() {
        let b = ResourceBudget::fixed(1 << 20).unwrap();
        let text = "Get a setting\n    Args:\n        key: Name.";
        let (s, e) = first_sentence(text, &b).unwrap().unwrap();
        assert_eq!(&text[s..e], "Get a setting");
        assert_eq!(first_sentence(" \n ", &b).unwrap(), None);
        let (b, d, _) = fixture("\"Run.\"", "Run.");
        let tiny = ResourceBudget::fixed(1).unwrap();
        assert!(matches!(build(&d, &tiny), Err(ModelError::Resource { .. })));
        assert_eq!(tiny.reserved(), 0);
        assert_eq!(build(&d, &b).unwrap().conclusions.len(), 1);
    }
}
