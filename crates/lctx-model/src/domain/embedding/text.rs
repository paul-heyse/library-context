//! Analytic text comes from normalized source declarations and original document observations.
//! Public catalog choices, synthesized briefs and retrieval units are deliberately not inputs.
use crate::domain::{
    artifact::{ARTIFACT_CHUNK_BYTES, ArtifactChunkKey},
    assertion::{AssertionQualification, Evidence},
    attribution::AnalysisContext,
    documents::PassageObservation,
    input::{ArtifactUse, InputRevision},
    normalized::{
        Rows,
        entities::{CallableEntity, CallableKind, ClassEntity, EntityRef},
    },
    resources::{Reservation, ResourceBudget},
    source::{Occurrence, SourceArtifact},
    syntax::{DeclarationKind, DeclarationObservation, ParameterSyntaxObservation, SyntaxDetail},
    *,
};
use crate::{Domain, DomainCode, DomainSum};

#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="analytic_text_definitions",validate=validate_definition)]
pub struct TextDefinition {
    #[model(key)]
    pub version: i32,
    #[model(key)]
    pub window_bytes: i64,
    #[model(key)]
    pub requested: bool,
}
impl TextDefinition {
    pub fn builtin() -> Self {
        Self {
            version: 2,
            window_bytes: 4096,
            requested: false,
        }
    }
}
fn validate_definition(row: &TextDefinition) -> Result<(), ModelError> {
    if row.version != 2
        || row.window_bytes < 4
        || row.window_bytes > resources::MAX_ROW_BYTES as i64
    {
        return Err(invalid("unsupported analytic text definition"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, DomainSum, serde::Serialize, serde::Deserialize)]
#[model(name = "analytic_text_subjects")]
pub enum TextSubject {
    #[model(code = 0)]
    Declaration {
        declaration: Id<DeclarationObservation>,
    },
    #[model(code = 1)]
    Passage { passage: Id<PassageObservation> },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TextAvailability {
    Available = 0,
    Unavailable = 1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum TextBoundary {
    MissingEntity = 0,
    MissingSource = 1,
    AmbiguousModule = 2,
    MissingParent = 3,
    MissingDocstring = 4,
    UnsupportedEncoding = 5,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="analytic_text_assessments",validate=validate_assessment,invariant_refs=text_invariants_refs)]
pub struct TextAssessment {
    #[model(key)]
    pub subject: Id<TextSubject>,
    #[model(key)]
    pub definition: Id<TextDefinition>,
    #[model(key)]
    pub input: Id<InputRevision>,
    #[model(key)]
    pub context: Id<AnalysisContext>,
    pub entity: Option<Id<EntityRef>>,
    pub source: Id<SourceArtifact>,
    pub availability: TextAvailability,
    pub boundary: Option<TextBoundary>,
}
fn validate_assessment(row: &TextAssessment) -> Result<(), ModelError> {
    if (row.availability == TextAvailability::Available) != row.boundary.is_none() {
        return Err(invalid("analytic text availability differs from boundary"));
    }
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="analytic_text_windows",validate=validate_window)]
pub struct TextWindow {
    #[model(key)]
    pub assessment: Id<TextAssessment>,
    #[model(key)]
    pub ordinal: i64,
    pub start: i64,
    pub end: i64,
    pub text: Utf8Text,
    pub content: ContentHash,
}
fn validate_window(row: &TextWindow) -> Result<(), ModelError> {
    if row.ordinal < 0
        || row.start < 0
        || row.end < row.start
        || (row.end - row.start) as usize != row.text.len()
        || row.content != ContentHash::of(row.text.as_bytes())
    {
        return Err(invalid("analytic text window bytes differ"));
    }
    Ok(())
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}

#[macro_export]
macro_rules! analytic_text_inputs {
    ($m:ident) => {
        $m! {
            artifacts:$crate::domain::source::SourceArtifact,
            uses:$crate::domain::input::ArtifactUse,
            chunks:$crate::domain::artifact::ArtifactChunk,
            modules:$crate::domain::source::Module,
            occurrences:$crate::domain::source::Occurrence,
            qualifications:$crate::domain::assertion::AssertionQualification,
            declarations:$crate::domain::syntax::DeclarationObservation,
            placements:$crate::domain::syntax::SyntaxPlacement,
            parameters:$crate::domain::syntax::ParameterSyntaxObservation,
            details:$crate::domain::syntax::SyntaxDetailObservation,
            detail_values:$crate::domain::syntax::SyntaxDetail,
            literals:$crate::domain::value::Literal,
            entities:$crate::domain::normalized::entities::EntityRef,
            callables:$crate::domain::normalized::entities::CallableEntity,
            classes:$crate::domain::normalized::entities::ClassEntity,
            passages:$crate::domain::documents::PassageObservation,
            nodes:$crate::domain::documents::DocumentNode,
            evidence:$crate::domain::assertion::Evidence,
        }
    };
}
macro_rules! inputs {($($field:ident:$ty:ty,)*)=>{
    pub struct TextData {$(pub $field:Rows<$ty>,)*}
    impl TextData {
        pub fn new(budget:&ResourceBudget)->Self {Self {$($field:Rows::new(budget),)*}}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
        pub fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
    }
};}
analytic_text_inputs!(inputs);
macro_rules! outputs {($($field:ident:$ty:ty,)*)=>{
    pub struct TextOutput {$(pub $field:Rows<$ty>,)*}
    impl TextOutput {
        pub fn new(budget:&ResourceBudget)->Self {Self {$($field:Rows::new(budget),)*}}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
        fn matches(&self,other:&Self)->Result<(),ModelError> {$(if !self.$field.same(&other.$field) {return Err(invalid(concat!("analytic text closure differs: ",stringify!($field))));})*Ok(())}
    }
};}
outputs! {definitions:TextDefinition,subjects:TextSubject,assessments:TextAssessment,windows:TextWindow,}
pub fn relations() -> Vec<Relation> {
    vec![
        Relation::of::<TextDefinition>(),
        Relation::of::<TextSubject>(),
        Relation::of::<TextAssessment>(),
        Relation::of::<TextWindow>(),
    ]
}

/// This owner needs only the earlier facts/normalized closure. Vocabulary is frozen at Facts;
/// no catalog, summary or retrieval publication can become an accidental text premise.
pub fn stage(
    profile: stages::Profile,
    definition: &TextDefinition,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    use stages::*;
    definition.validate()?;
    let earlier = normalized_relations()
        .iter()
        .map(Relation::name)
        .collect::<std::collections::BTreeSet<_>>();
    let outputs = relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    let inputs = dependency_closure::DependencyClosure::stage_grants(
        model,
        TextData::inputs(),
        &outputs,
        PublicationBoundary::Facts,
        dependency_closure::LowerLayerPolicy::IncludeInferredOrdinaryFacts,
        order,
    )?;
    if inputs.iter().any(|input| !earlier.contains(input.name())) {
        return Err(invalid(
            "analytic text premise is not facts or normalized authority",
        ));
    }
    let mut key = KeySink::new("analytic-source-text-v2");
    definition.id().encode(&mut key);
    Ok(Stage {
        name: "analytic_text",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("text.rs")),
        configuration: key.finish(),
    })
}

struct Text {
    value: String,
    _reservation: Box<dyn Reservation>,
}
impl TextData {
    fn context(&self, q: Id<AssertionQualification>) -> Result<Id<AnalysisContext>, ModelError> {
        self.qualifications
            .get(q)
            .map(|q| q.context)
            .ok_or_else(|| invalid("analytic source qualification absent"))
    }
    fn span(
        &self,
        occurrence: Id<Occurrence>,
        budget: &ResourceBudget,
    ) -> Result<Result<Text, TextBoundary>, ModelError> {
        let Some(span) = self.occurrences.get(occurrence) else {
            return Ok(Err(TextBoundary::MissingSource));
        };
        let Some(artifact) = self.artifacts.get(span.source) else {
            return Ok(Err(TextBoundary::MissingSource));
        };
        if span.start < 0 || span.end < span.start || span.end > artifact.byte_len {
            return Err(invalid("analytic source span is outside artifact"));
        }
        let len = (span.end - span.start) as usize;
        let reservation = budget.reserve("analytic-source-span", len + size_of::<Text>())?;
        let mut bytes = Vec::with_capacity(len);
        let mut offset = span.start as usize;
        while offset < span.end as usize {
            let ordinal = (offset / ARTIFACT_CHUNK_BYTES) as i64;
            let id = Id::of(&ArtifactChunkKey {
                artifact: artifact.id(),
                ordinal,
            });
            let Some(chunk) = self.chunks.get(id) else {
                return Ok(Err(TextBoundary::MissingSource));
            };
            let first = offset % ARTIFACT_CHUNK_BYTES;
            let count = (span.end as usize - offset).min(ARTIFACT_CHUNK_BYTES - first);
            let Some(part) = chunk.body.0.get(first..first + count) else {
                return Ok(Err(TextBoundary::MissingSource));
            };
            bytes.extend_from_slice(part);
            offset += count;
        }
        match String::from_utf8(bytes) {
            Ok(value) => Ok(Ok(Text {
                value,
                _reservation: reservation,
            })),
            Err(_) => Ok(Err(TextBoundary::UnsupportedEncoding)),
        }
    }
    fn declaration_text(
        &self,
        row: &DeclarationObservation,
        budget: &ResourceBudget,
    ) -> Result<Result<Text, TextBoundary>, ModelError> {
        let context = self.context(row.qualification)?;
        let source = self
            .occurrences
            .get(row.declaration)
            .ok_or_else(|| invalid("analytic declaration occurrence absent"))?
            .source;
        let mut modules = self.modules.iter().filter(|m| m.source == source);
        let Some(module) = modules.next() else {
            return Ok(Err(TextBoundary::MissingSource));
        };
        if modules.next().is_some() {
            return Ok(Err(TextBoundary::AmbiguousModule));
        }
        let mut charge = charged::StateCharge::new(budget, "analytic-declaration-render");
        let mut names = Vec::<Text>::new();
        let mut current = row;
        let mut count = 0;
        loop {
            charge.grow(size_of::<Text>() * 2)?;
            match self.span(current.name, budget)? {
                Ok(name) => names.push(name),
                Err(boundary) => return Ok(Err(boundary)),
            }
            let Some(parent) = current.parent else { break };
            count += 1;
            if count > self.declarations.len() {
                return Err(invalid("cyclic analytic declaration ancestry"));
            }
            let mut parents = self.declarations.iter().filter(|candidate| {
                candidate.declaration == parent
                    && self.context(candidate.qualification).ok() == Some(context)
            });
            let Some(parent) = parents.next() else {
                return Ok(Err(TextBoundary::MissingParent));
            };
            if parents.next().is_some() {
                return Ok(Err(TextBoundary::MissingParent));
            }
            current = parent;
        }
        let mut parameters = Vec::<(&ParameterSyntaxObservation, Text)>::new();
        for parameter in self.parameters.iter().filter(|p| {
            p.function == row.declaration && self.context(p.qualification).ok() == Some(context)
        }) {
            charge.grow(size_of::<(&ParameterSyntaxObservation, Text)>() * 2)?;
            match self.span(parameter.parameter, budget)? {
                Ok(text) => parameters.push((parameter, text)),
                Err(boundary) => return Ok(Err(boundary)),
            }
        }
        parameters.sort_by_key(|(p, _)| (p.ordinal, p.id()));
        if parameters
            .windows(2)
            .any(|p| p[0].0.ordinal == p[1].0.ordinal)
        {
            return Err(invalid("ambiguous analytic parameter ordinal"));
        }
        let doc =
            if let Some(docstring) = row.docstring {
                // DeclarationObservation names the source expression statement, not its literal.
                // Follow its exact typed Value edge rather than matching equal byte spans.
                let Some(statement) = self.occurrences.get(docstring).filter(|s| {
                    s.syntax_kind == source::SyntaxKind::StmtExpr && s.source == source
                }) else {
                    return Ok(Err(TextBoundary::MissingDocstring));
                };
                let mut children = self
                    .placements
                    .iter()
                    .filter(|p| {
                        p.parent == Some(statement.id())
                            && p.field == lexical::SyntaxField::Value
                            && p.ordinal == 0
                            && self.context(p.qualification).ok() == Some(context)
                    })
                    .filter_map(|p| self.occurrences.get(p.occurrence))
                    .filter(|child| {
                        child.syntax_kind == source::SyntaxKind::ExprStringLiteral
                            && child.source == source
                    });
                let Some(literal) = children.next() else {
                    return Ok(Err(TextBoundary::MissingDocstring));
                };
                if children.any(|other| other.id() != literal.id()) {
                    return Ok(Err(TextBoundary::MissingDocstring));
                }
                let mut values = self
                    .details
                    .iter()
                    .filter(|d| {
                        d.occurrence == literal.id()
                            && self.context(d.qualification).ok() == Some(context)
                    })
                    .filter_map(|d| match self.detail_values.get(d.detail) {
                        Some(SyntaxDetail::Literal { literal }) => self.literals.get(*literal),
                        _ => None,
                    })
                    .filter_map(|literal| match literal {
                        value::Literal::String { value } => Some(value.as_str()),
                        _ => None,
                    });
                let Some(doc) = values.next() else {
                    return Ok(Err(TextBoundary::MissingDocstring));
                };
                if values.any(|other| other != doc) {
                    return Ok(Err(TextBoundary::MissingDocstring));
                }
                Some(doc)
            } else {
                None
            };
        let capacity = module.qualified_name.len()
            + names.iter().map(|n| n.value.len() + 1).sum::<usize>()
            + parameters
                .iter()
                .map(|(_, t)| t.value.len() + 5)
                .sum::<usize>()
            + doc.map_or(0, str::len)
            + 8;
        let reservation = budget.reserve("analytic-rendered-text", capacity + size_of::<Text>())?;
        let mut text = String::with_capacity(capacity);
        text.push_str(&module.qualified_name);
        for name in names.iter().rev() {
            text.push('.');
            text.push_str(&name.value);
        }
        text.push('(');
        let mut has_star = false;
        for (index, (parameter, value)) in parameters.iter().enumerate() {
            use calls::ParameterKind::*;
            if index > 0 {
                text.push_str(", ");
            }
            if parameter.kind == KeywordOnly && !has_star {
                text.push_str("*, ");
                has_star = true;
            }
            // The captured parameter span already includes its * or ** marker.
            if parameter.kind == VarPositional {
                has_star = true;
            }
            text.push_str(&value.value);
            if parameter.kind == PositionalOnly
                && parameters
                    .get(index + 1)
                    .is_none_or(|(next, _)| next.kind != PositionalOnly)
            {
                text.push_str(", /");
            }
        }
        text.push(')');
        if let Some(doc) = doc.map(str::trim).filter(|d| !d.is_empty()) {
            text.push('\n');
            text.push_str(doc);
        }
        Ok(Ok(Text {
            value: text,
            _reservation: reservation,
        }))
    }
}

/// The preserved line-end/UTF-8 window operation returns borrowed slices. Its index allocation
/// is charged, and byte windows never stand in for the service's later tokenizer admission.
pub struct Windows<'a> {
    rows: Vec<(usize, &'a str)>,
    _reservation: Box<dyn Reservation>,
}
impl<'a> Windows<'a> {
    pub fn iter(&self) -> impl Iterator<Item = (usize, &'a str)> + '_ {
        self.rows.iter().copied()
    }
}
pub fn windows<'a>(
    text: &'a str,
    cap: usize,
    budget: &ResourceBudget,
) -> Result<Windows<'a>, ModelError> {
    if cap < 4 {
        return Err(invalid(
            "analytic text window cap must hold one Unicode scalar",
        ));
    }
    let bound = text
        .split_inclusive('\n')
        .count()
        .checked_add(text.len() / cap.saturating_sub(3))
        .and_then(|n| n.checked_add(2))
        .ok_or_else(|| invalid("analytic window allocation overflow"))?;
    let reservation = budget.reserve(
        "analytic-window-index",
        bound
            .checked_mul(size_of::<(usize, &str)>())
            .ok_or_else(|| invalid("analytic window allocation overflow"))?,
    )?;
    let mut rows = Vec::with_capacity(bound);
    let mut start = 0;
    let mut end = 0;
    for line in text.split_inclusive('\n') {
        if end - start + line.len() <= cap {
            end += line.len();
            continue;
        }
        if end > start {
            rows.push((start, &text[start..end]));
            start = end;
        }
        let line_end = end + line.len();
        while line_end - start > cap {
            let mut cut = start + cap;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            rows.push((start, &text[start..cut]));
            start = cut;
        }
        end = line_end;
    }
    if end > start || rows.is_empty() {
        rows.push((start, &text[start..end]));
    }
    Ok(Windows {
        rows,
        _reservation: reservation,
    })
}
fn emit_text(
    out: &mut TextOutput,
    assessment: TextAssessment,
    text: Option<&str>,
    definition: &TextDefinition,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    if let Some(text) = text {
        for (ordinal, (start, part)) in windows(text, definition.window_bytes as usize, budget)?
            .iter()
            .enumerate()
        {
            out.windows.insert(TextWindow {
                assessment: assessment.id(),
                ordinal: ordinal as i64,
                start: start as i64,
                end: (start + part.len()) as i64,
                text: part.into(),
                content: ContentHash::of(part.as_bytes()),
            })?;
        }
    }
    out.assessments.insert(assessment)?;
    Ok(())
}
pub fn prepare(
    data: &TextData,
    definition: &TextDefinition,
    budget: &ResourceBudget,
) -> Result<TextOutput, ModelError> {
    definition.validate()?;
    let mut out = TextOutput::new(budget);
    out.definitions.insert(definition.clone())?;
    if !definition.requested {
        return Ok(out);
    }
    // Captured use, not a coincidental path, selects source material. Dependency context stays out.
    let root_bytes = data
        .artifacts
        .iter()
        .map(|r| size_of::<SourceArtifact>() + r.heap_bytes() + 96)
        .sum::<usize>()
        + data.uses.len() * (size_of::<ArtifactUse>() + 64);
    let _roots = budget.reserve("analytic-text-roots", root_bytes)?;
    let roots = admission::analysis_roots(
        &data.artifacts.iter().cloned().collect::<Vec<_>>(),
        &data.uses.iter().cloned().collect::<Vec<_>>(),
    )?;
    for declaration in data.declarations.iter() {
        let source = data
            .occurrences
            .get(declaration.declaration)
            .ok_or_else(|| invalid("analytic declaration occurrence absent"))?
            .source;
        if !roots.contains(&source) {
            continue;
        }
        let artifact = data
            .artifacts
            .get(source)
            .ok_or_else(|| invalid("analytic declaration artifact absent"))?;
        let context = data.context(declaration.qualification)?;
        let subject = out.subjects.insert(TextSubject::Declaration {
            declaration: declaration.id(),
        })?;
        let entity = match declaration.kind {
            DeclarationKind::Class => {
                let class = ClassEntity::Source {
                    declaration: declaration.declaration,
                };
                data.classes
                    .get(class.id())
                    .map(|_| EntityRef::Class { class: class.id() })
            }
            _ => {
                let callable = CallableEntity::Source {
                    declaration: declaration.declaration,
                    kind: CallableKind::Function,
                };
                data.callables
                    .get(callable.id())
                    .map(|_| EntityRef::Callable {
                        callable: callable.id(),
                    })
            }
        }
        .and_then(|entity| data.entities.get(entity.id()).map(Record::id));
        let rendered = if entity.is_none() {
            Err(TextBoundary::MissingEntity)
        } else {
            data.declaration_text(declaration, budget)?
        };
        let assessment = TextAssessment {
            subject,
            definition: definition.id(),
            input: artifact.input,
            context,
            entity,
            source,
            availability: if rendered.is_ok() {
                TextAvailability::Available
            } else {
                TextAvailability::Unavailable
            },
            boundary: rendered.as_ref().err().copied(),
        };
        emit_text(
            &mut out,
            assessment,
            rendered.as_ref().ok().map(|t| t.value.as_str()),
            definition,
            budget,
        )?;
    }
    for passage in data.passages.iter() {
        let node = data
            .nodes
            .get(passage.passage.id())
            .ok_or_else(|| invalid("analytic passage node absent"))?;
        let source = match data.evidence.get(node.span().id()) {
            Some(Evidence::SourceSpan { source, .. }) => *source,
            _ => return Err(invalid("analytic passage original span absent")),
        };
        if !roots.contains(&source) {
            continue;
        }
        let artifact = data
            .artifacts
            .get(source)
            .ok_or_else(|| invalid("analytic passage artifact absent"))?;
        let subject = out.subjects.insert(TextSubject::Passage {
            passage: passage.id(),
        })?;
        emit_text(
            &mut out,
            TextAssessment {
                subject,
                definition: definition.id(),
                input: artifact.input,
                context: data.context(passage.qualification)?,
                entity: None,
                source,
                availability: TextAvailability::Available,
                boundary: None,
            },
            Some(&passage.text),
            definition,
            budget,
        )?;
    }
    Ok(out)
}
pub(crate) fn text_invariants() -> Vec<Invariant> {
    let mut inputs = TextData::inputs();
    inputs.extend([
        ValidationInput::of::<TextDefinition>(&["id"]),
        ValidationInput::of::<TextSubject>(&["id"]),
        ValidationInput::of::<TextAssessment>(&["id"]),
        ValidationInput::of::<TextWindow>(&["id"]),
    ]);
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::DiagnosticReplay,
        revision: 1,
        name: "analytic_text_replay",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(TextCheck {
                data: TextData::new(budget),
                actual: TextOutput::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct TextCheck {
    data: TextData,
    actual: TextOutput,
    budget: ResourceBudget,
}
impl InvariantCheck for TextCheck {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.data.visit(name, batch)? || self.actual.visit(name, batch)? {
            Ok(())
        } else {
            Err(invalid("undeclared analytic text input"))
        }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        if self.actual.definitions.len() != 1 {
            return Err(invalid("analytic text needs exactly one definition"));
        }
        let expected = prepare(
            &self.data,
            self.actual
                .definitions
                .iter()
                .next()
                .expect("one definition"),
            &self.budget,
        )?;
        self.actual.matches(&expected)
    }
}

pub(crate) fn text_invariants_refs() -> Vec<&'static str> {
    vec!["analytic_text_replay"]
}
