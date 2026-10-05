//! Document structure refers to captured byte spans. Interpretations remain qualified assertions;
//! expressions and spreads retain source text and are never evaluated by the facts layer.
use super::charged::{ChargedMap, ChargedSet, ChargedVec, StateCharge};
use super::{
    assertion::{AssertionQualification, EvidenceSourceSpanId},
    attribution::FactFamily,
    source::SourceArtifact,
    *,
};
use crate::{Assertion, Domain, DomainCode, DomainSum};

#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "document_nodes", validate = validate_node, invariant_refs = document_invariants_refs)]
pub enum DocumentNode {
    #[model(code = 0)]
    Passage {
        span: EvidenceSourceSpanId,
        ordinal: i64,
    },
    #[model(code = 1)]
    CodeBlock {
        span: EvidenceSourceSpanId,
        ordinal: i64,
    },
    #[model(code = 2)]
    Link {
        span: EvidenceSourceSpanId,
        ordinal: i64,
    },
    #[model(code = 3)]
    Mention { span: EvidenceSourceSpanId },
    #[model(code = 4)]
    Component {
        span: EvidenceSourceSpanId,
        ordinal: i64,
    },
}
impl DocumentNode {
    pub fn span(&self) -> EvidenceSourceSpanId {
        match self {
            Self::Passage { span, .. }
            | Self::CodeBlock { span, .. }
            | Self::Link { span, .. }
            | Self::Mention { span }
            | Self::Component { span, .. } => *span,
        }
    }
    pub fn ordinal(&self) -> Option<i64> {
        match self {
            Self::Passage { ordinal, .. }
            | Self::CodeBlock { ordinal, .. }
            | Self::Link { ordinal, .. }
            | Self::Component { ordinal, .. } => Some(*ordinal),
            Self::Mention { .. } => None,
        }
    }
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_node(row: &DocumentNode) -> Result<(), ModelError> {
    if row.ordinal().is_some_and(|n| n < 0) {
        return Err(invalid("document ordinal must be nonnegative"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "document_observations")]
#[assertion(support = DocumentSupport, name = "document_supports", family = FactFamily::Docs, subjects(source))]
pub struct DocumentObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub source: Id<SourceArtifact>,
    #[model(key)]
    pub title: Option<String>,
    #[model(key)]
    pub parsed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "passage_observations", validate = validate_passage)]
#[assertion(support = PassageSupport, name = "passage_supports", family = FactFamily::Docs, subjects(passage))]
pub struct PassageObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub passage: DocumentNodePassageId,
    #[model(key)]
    pub level: i64,
    #[model(key)]
    pub heading: Option<String>,
    /// Enclosing headings, excluding this passage's own heading.
    #[model(key)]
    pub heading_path: Vec<String>,
    #[model(key)]
    pub text: String,
}
fn validate_passage(row: &PassageObservation) -> Result<(), ModelError> {
    if !(0..=6).contains(&row.level)
        || (row.level == 0 && (row.heading.is_some() || !row.heading_path.is_empty()))
    {
        return Err(invalid("invalid passage heading level or preamble"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "code_block_observations", validate = validate_code)]
#[assertion(support = CodeBlockSupport, name = "code_block_supports", family = FactFamily::Docs, subjects(block, passage), referents(materialized))]
pub struct CodeBlockObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub block: DocumentNodeCodeBlockId,
    #[model(key)]
    pub passage: DocumentNodePassageId,
    #[model(key)]
    pub language: Option<String>,
    #[model(key)]
    pub meta: Option<String>,
    #[model(key)]
    pub code: String,
    #[model(key)]
    pub content: ContentHash,
    #[model(key)]
    pub module_path: Option<String>,
    /// Captured derived code bytes, checked against the parsed code rather than inferred by path.
    #[model(key)]
    pub materialized: Option<Id<SourceArtifact>>,
}
fn validate_code(row: &CodeBlockObservation) -> Result<(), ModelError> {
    if row.content != ContentHash::of(row.code.as_bytes()) {
        return Err(invalid("code block content differs from captured code"));
    }
    if row.module_path.is_some() != row.materialized.is_some() {
        return Err(invalid(
            "code module path and captured materialization agree",
        ));
    }
    if let Some(path) = &row.module_path {
        super::input::validate_path(path)?;
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "document_link_observations")]
#[assertion(support = DocumentLinkSupport, name = "document_link_supports", family = FactFamily::Docs, subjects(link, passage))]
pub struct DocumentLinkObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub link: DocumentNodeLinkId,
    #[model(key)]
    pub passage: DocumentNodePassageId,
    #[model(key)]
    pub url: String,
    #[model(key)]
    pub title: Option<String>,
    #[model(key)]
    pub text: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum MentionClass {
    Exact = 0,
    Lexical = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum MentionSource {
    InlineCode = 0,
    Prose = 1,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "document_mention_observations")]
#[assertion(support = DocumentMentionSupport, name = "document_mention_supports", family = FactFamily::Docs, subjects(mention, passage))]
pub struct DocumentMentionObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub mention: DocumentNodeMentionId,
    #[model(key)]
    pub passage: DocumentNodePassageId,
    #[model(key)]
    pub class: MentionClass,
    #[model(key)]
    pub source: MentionSource,
    #[model(key)]
    pub form: String,
    /// Observed vocabulary spellings; cross-provider/entity equivalence belongs to normalization.
    #[model(key)]
    pub access_path: Option<String>,
    #[model(key)]
    pub qualified_name: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum ComponentForm {
    Flow = 0,
    Text = 1,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "document_component_observations", validate = validate_component)]
#[assertion(support = DocumentComponentSupport, name = "document_component_supports", family = FactFamily::Docs, subjects(component, passage, parent, inner, lead))]
pub struct DocumentComponentObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub component: DocumentNodeComponentId,
    #[model(key)]
    pub passage: DocumentNodePassageId,
    #[model(key)]
    pub parent: Option<DocumentNodeComponentId>,
    #[model(key)]
    pub depth: i64,
    #[model(key)]
    pub name: Option<String>,
    #[model(key)]
    pub form: ComponentForm,
    #[model(key)]
    pub inner: Option<EvidenceSourceSpanId>,
    #[model(key)]
    pub lead: Option<EvidenceSourceSpanId>,
}
fn validate_component(row: &DocumentComponentObservation) -> Result<(), ModelError> {
    if row.depth < 0
        || (row.depth == 0) != row.parent.is_none()
        || row.parent == Some(row.component)
    {
        return Err(invalid("invalid component parent/depth"));
    }
    Ok(())
}
/// Codes retain the old attribute codebook. Payload shape, rather than a parallel optional-field
/// convention, distinguishes literal content, expression source, a bare name and a spread.
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum)]
#[model(name = "document_attribute_values", validate = validate_attribute)]
pub enum DocumentAttributeValue {
    #[model(code = 0)]
    Literal { name: String, value: String },
    #[model(code = 1)]
    Expression { name: String, source: String },
    #[model(code = 2)]
    Bare { name: String },
    #[model(code = 3)]
    Spread { source: String },
}
fn validate_attribute(row: &DocumentAttributeValue) -> Result<(), ModelError> {
    if matches!(row,DocumentAttributeValue::Literal { name,.. } | DocumentAttributeValue::Expression { name,.. } | DocumentAttributeValue::Bare { name } if name.is_empty())
    {
        return Err(invalid("named component attribute needs a name"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion, serde::Serialize, serde::Deserialize)]
#[model(name = "document_attribute_observations", validate = validate_attribute_observation)]
#[assertion(support = DocumentAttributeSupport, name = "document_attribute_supports", family = FactFamily::Docs, subjects(component))]
pub struct DocumentAttributeObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub component: DocumentNodeComponentId,
    #[model(key)]
    pub ordinal: i64,
    #[model(key)]
    pub value: Id<DocumentAttributeValue>,
}
fn validate_attribute_observation(row: &DocumentAttributeObservation) -> Result<(), ModelError> {
    if row.ordinal < 0 {
        return Err(invalid("attribute ordinal must be nonnegative"));
    }
    Ok(())
}

pub(crate) fn document_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "document_structure",
        inputs: vec![
            ValidationInput::of::<SourceArtifact>(&["id"]),
            ValidationInput::of::<super::assertion::Evidence>(&["id"]),
            ValidationInput::of::<DocumentNode>(&["id"]),
            ValidationInput::of::<PassageObservation>(&["id"]),
            ValidationInput::of::<CodeBlockObservation>(&["id"]),
            ValidationInput::of::<DocumentLinkObservation>(&["id"]),
            ValidationInput::of::<DocumentMentionObservation>(&["id"]),
            ValidationInput::of::<DocumentComponentObservation>(&["id"]),
            ValidationInput::of::<DocumentAttributeObservation>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(DocumentCheck {
                charge: StateCharge::new(budget, "document_structure"),
                ..Default::default()
            })
        }),
    }]
}
#[derive(Debug, Clone, Copy)]
struct Span {
    source: Id<SourceArtifact>,
    start: i64,
    end: i64,
}
impl HeapSize for Span {}
impl Span {
    fn contains(self, child: Self) -> bool {
        self.source == child.source && self.start <= child.start && self.end >= child.end
    }
}
#[derive(Default)]
struct DocumentCheck {
    charge: StateCharge,
    artifacts: ChargedMap<Id<SourceArtifact>, SourceArtifact>,
    spans: ChargedMap<Id<super::assertion::Evidence>, Span>,
    nodes: ChargedMap<Id<DocumentNode>, DocumentNode>,
    components: ChargedVec<DocumentComponentObservation>,
}
impl DocumentCheck {
    fn span(&self, id: EvidenceSourceSpanId) -> Result<Span, ModelError> {
        self.spans
            .get(&id.id())
            .copied()
            .ok_or_else(|| invalid("document source span missing or wrong evidence subtype"))
    }
    fn node<const C: i16>(&self, id: ArmId<DocumentNode, C>) -> Result<&DocumentNode, ModelError> {
        self.nodes
            .get(&id.id())
            .filter(|n| n.tag() == C)
            .ok_or_else(|| invalid("document node missing or wrong subtype"))
    }
    fn inside<const C: i16>(
        &self,
        child: ArmId<DocumentNode, C>,
        passage: DocumentNodePassageId,
    ) -> Result<(), ModelError> {
        let owner = self.span(self.node(passage)?.span())?;
        let child = self.span(self.node(child)?.span())?;
        if owner.source != child.source || child.start < owner.start || child.start >= owner.end {
            return Err(invalid("document node start lies outside its passage"));
        }
        Ok(())
    }
}
impl InvariantCheck for DocumentCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        use super::assertion::Evidence;
        if relation == SourceArtifact::NAME {
            for row in SourceArtifact::decode(batch)? {
                self.artifacts.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == Evidence::NAME {
            for row in Evidence::decode(batch)? {
                if let Evidence::SourceSpan { source, start, end } = &row {
                    self.spans.insert(
                        &mut self.charge,
                        row.id(),
                        Span {
                            source: *source,
                            start: *start,
                            end: *end,
                        },
                    )?;
                }
            }
        } else if relation == DocumentNode::NAME {
            for row in DocumentNode::decode(batch)? {
                self.span(row.span())?;
                self.nodes.insert(&mut self.charge, row.id(), row)?;
            }
        } else if relation == PassageObservation::NAME {
            for row in PassageObservation::decode(batch)? {
                let span = self.span(self.node(row.passage)?.span())?;
                if row.text.len() as i64 != span.end - span.start {
                    return Err(invalid("passage text length differs from original span"));
                }
            }
        } else if relation == CodeBlockObservation::NAME {
            for row in CodeBlockObservation::decode(batch)? {
                self.inside(row.block, row.passage)?;
                if let Some(id) = row.materialized {
                    let block = self.span(self.node(row.block)?.span())?;
                    let source = self
                        .artifacts
                        .get(&block.source)
                        .ok_or_else(|| invalid("code source absent"))?;
                    let derived = self
                        .artifacts
                        .get(&id)
                        .ok_or_else(|| invalid("materialized code artifact absent"))?;
                    if derived.input != source.input
                        || derived.content != row.content
                        || derived.byte_len != row.code.len() as i64
                        || row.module_path.as_deref() != Some(derived.path.as_str())
                    {
                        return Err(invalid(
                            "materialized code differs from parsed block or captured input",
                        ));
                    }
                }
            }
        } else if relation == DocumentLinkObservation::NAME {
            for row in DocumentLinkObservation::decode(batch)? {
                self.inside(row.link, row.passage)?;
            }
        } else if relation == DocumentMentionObservation::NAME {
            for row in DocumentMentionObservation::decode(batch)? {
                self.inside(row.mention, row.passage)?;
            }
        } else if relation == DocumentComponentObservation::NAME {
            for row in DocumentComponentObservation::decode(batch)? {
                self.inside(row.component, row.passage)?;
                let component = self.node(row.component)?;
                let span = self.span(component.span())?;
                for part in [row.inner, row.lead].into_iter().flatten() {
                    if !span.contains(self.span(part)?) {
                        return Err(invalid("component inner/lead span outside component"));
                    }
                }
                if let Some(parent) = row.parent {
                    let parent = self.node(parent)?;
                    if parent.ordinal() >= component.ordinal()
                        || !self.span(parent.span())?.contains(span)
                    {
                        return Err(invalid("component parent must precede and enclose child"));
                    }
                }
                self.components.push(&mut self.charge, row)?;
            }
        } else if relation == DocumentAttributeObservation::NAME {
            for row in DocumentAttributeObservation::decode(batch)? {
                self.node(row.component)?;
            }
        } else {
            return Err(invalid("undeclared document validation input"));
        }
        Ok(())
    }
    fn finish(mut self: Box<Self>) -> Result<(), ModelError> {
        let mut depths = ChargedSet::default();
        for row in self.components.iter() {
            depths.insert(
                &mut self.charge,
                (row.qualification, row.component.id(), row.depth),
            )?;
        }
        for row in self.components.iter() {
            if let Some(parent) = row.parent
                && !depths.contains(&(row.qualification, parent.id(), row.depth - 1))
            {
                return Err(invalid("component parent depth or qualification differs"));
            }
        }
        Ok(())
    }
}

pub(crate) fn document_invariants_refs() -> Vec<&'static str> {
    vec!["document_structure"]
}
