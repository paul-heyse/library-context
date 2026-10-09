//! Source-call and enriched execution demand, with explicit private body namespaces.
use super::{
    catalog_scope_program::{BudgetedCatalogProgram, Builder},
    scope_program::*,
    *,
};
use calls::*;
use declarations::{ParameterDeclaration, SymbolDeclaration};
use execution::{body_records::*, records::*};
use flow::*;
use lexical::*;
use normalized::{bindings::*, callables::*, entities::*, events::*};
use source::*;
use std::any::TypeId;
use symbols::*;
use syntax::*;
fn c(row: usize, field: &'static str) -> ScopeColumn {
    ScopeColumn { row, field }
}
fn eq(a: usize, af: &'static str, b: usize, bf: &'static str) -> ScopePredicate {
    ScopePredicate::Equal(c(a, af), c(b, bf))
}
pub struct SourceCallPorts {
    pub callee: usize,
    pub header: usize,
    pub payload: usize,
    pub children: usize,
    pub owner: Option<usize>,
    pub statement: Option<usize>,
}
pub fn build(
    inputs: Vec<ValidationInput>,
    input_relations: &[Relation],
    real: usize,
    ports: SourceCallPorts,
    budget: &resources::ResourceBudget,
) -> Result<BudgetedCatalogProgram, ModelError> {
    let index = |kind: TypeId| {
        input_relations[..real]
            .iter()
            .position(|r| r.type_id() == kind)
            .ok_or(ModelError::Schema("SourceCall dependency input"))
    };
    let occurrence = index(TypeId::of::<Occurrence>())?;
    let SourceCallPorts {
        callee,
        header,
        payload,
        children,
        owner,
        statement,
    } = ports;
    let mut b = Builder::new(inputs.clone(), real, input_relations, budget)?;
    for (source, r) in input_relations[..real].iter().enumerate() {
        for field in r.fields() {
            if (r.type_id() == TypeId::of::<DeclarationObservation>()
                && field.name() == "docstring")
                || (r.type_id() == TypeId::of::<SyntaxDetail>()
                    && field.name() == "literal_literal")
            {
                continue;
            }
            let Some((kind, _)) = field.target() else {
                continue;
            };
            let Some(target) = scope_program::field_target(&inputs[..real], source, kind)? else {
                continue;
            };
            b.follow(source, field.name(), target, field.list());
            if field.name() == "assertion"
                || field.name().ends_with("_assertion")
                || (r.type_id() == TypeId::of::<analysis::native::NativeQualification>()
                    && field.name() == "premise")
            {
                b.reverse(source, field.name(), target);
            }
        }
    }
    macro_rules! own {
        ($member:ty,$field:literal,$owner:ty) => {
            b.own(
                index(TypeId::of::<$member>())?,
                $field,
                index(TypeId::of::<$owner>())?,
            )
        };
    }
    own!(input::ArtifactUse, "artifact", SourceArtifact);
    own!(OccurrenceOwnership, "occurrence", Occurrence);
    own!(SyntaxPlacement, "occurrence", Occurrence);
    own!(DeclarationObservation, "declaration", Occurrence);
    own!(SyntaxObservation, "occurrence", Occurrence);
    own!(NormalizedCallAlternative, "event", NormalizedCallEvent);
    own!(CallBindingAttempt, "event", NormalizedCallEvent);
    own!(CallBinding, "attempt", CallBindingAttempt);
    own!(BindingSetAssessment, "event", NormalizedCallEvent);
    own!(BindingVariantAssessment, "set", BindingSetAssessment);
    own!(BindingSetMember, "variant", BindingVariantAssessment);
    own!(BindingSetCoverage, "set", BindingSetAssessment);
    own!(
        EffectiveCallableEvidence,
        "assessment",
        EffectiveCallableAssessment
    );
    own!(EffectiveCallableAssessment, "callable", CallableEntity);
    own!(SignatureVariant, "signature", Signature);
    own!(SignatureSlot, "variant", SignatureVariant);
    own!(SignatureSlotEntity, "slot", SignatureSlot);
    own!(SignatureParameter, "signature", Signature);
    own!(ParameterEntityLink, "parameter", SignatureParameter);
    own!(ParameterDeclaration, "parameter", SignatureParameter);
    own!(
        SignatureEnumerationMember,
        "enumeration",
        SignatureEnumerationObservation
    );
    own!(CallArgument, "call", CallSyntax);
    own!(CallSyntax, "site", Occurrence);
    own!(CallTarget, "site", Occurrence);
    own!(ReferenceObservation, "read", Occurrence);
    own!(LexicalResolution, "read", Occurrence);
    own!(BindingEvent, "site", Occurrence);
    own!(BindingObservation, "event", BindingEvent);
    own!(FlowUse, "occurrence", Occurrence);
    own!(FlowUseObservation, "use_", FlowUse);
    own!(FlowReachingObservation, "use_", FlowUse);
    own!(FlowValueObservation, "use_", FlowUse);
    own!(FlowDefinition, "occurrence", Occurrence);
    own!(FlowDefinitionObservation, "definition", FlowDefinition);
    own!(ExpressionEvaluation, "expression", Occurrence);
    own!(EvaluationMember, "evaluation", ExpressionEvaluation);
    own!(EvaluationOperand, "evaluation", ExpressionEvaluation);
    own!(BodyMember, "body", SourceBodyCompletion);
    own!(BodyReleaseInput, "body", SourceBodyCompletion);
    own!(LexicalScope, "owner", Occurrence);
    own!(SymbolDeclaration, "declaration", Occurrence);
    own!(SymbolDeclaration, "symbol", ProviderSymbol);
    own!(SymbolObservation, "symbol", ProviderSymbol);
    own!(SymbolSequenceMember, "sequence", SymbolSequence);

    b.follow(
        index(TypeId::of::<DeclarationObservation>())?,
        "docstring",
        occurrence,
        false,
    );
    let alternatives = index(TypeId::of::<NormalizedCallAlternative>())?;
    let refs = index(TypeId::of::<EntityRef>())?;
    let callables = index(TypeId::of::<CallableEntity>())?;
    let placements = index(TypeId::of::<SyntaxPlacement>())?;
    let declarations = index(TypeId::of::<DeclarationObservation>())?;
    let bodies = index(TypeId::of::<SourceBodyCompletion>())?;
    let events = index(TypeId::of::<NormalizedCallEvent>())?;
    let frames = index(TypeId::of::<analysis::base_completion::AnalysisInvocation>())?;
    b.pair(
        alternatives,
        callee,
        &[alternatives],
        vec![ScopePredicate::IsNull(c(0, "entity"), false)],
        c(0, "id"),
        c(0, "id"),
    );
    b.pair(
        callee,
        refs,
        &[alternatives],
        vec![ScopePredicate::IsNull(c(0, "entity"), false)],
        c(0, "id"),
        c(0, "entity"),
    );
    b.pair(
        callee,
        bodies,
        &[alternatives, events, bodies, frames],
        vec![
            eq(1, "id", 0, "event"),
            eq(2, "owner", 0, "entity"),
            eq(3, "id", 2, "invocation"),
            eq(3, "context", 1, "context"),
        ],
        c(0, "id"),
        c(2, "id"),
    );
    b.pair(
        callee,
        header,
        &[alternatives, refs, callables],
        vec![
            eq(1, "id", 0, "entity"),
            eq(2, "id", 1, "callable_callable"),
            ScopePredicate::IsNull(c(2, "source_declaration"), false),
        ],
        c(0, "id"),
        c(2, "source_declaration"),
    );
    b.pair(
        header,
        occurrence,
        &[occurrence],
        vec![],
        c(0, "id"),
        c(0, "id"),
    );
    b.pair(
        header,
        children,
        &[occurrence, placements],
        vec![eq(1, "parent", 0, "id")],
        c(0, "id"),
        c(1, "id"),
    );
    let resolutions = index(TypeId::of::<LexicalResolution>())?;
    b.pair(
        header,
        resolutions,
        &[occurrence, occurrence, resolutions],
        vec![
            ScopePredicate::BodyContains {
                parent: 0,
                child: 1,
            },
            eq(2, "read", 1, "id"),
            ScopePredicate::Boolean(c(2, "captured"), true),
        ],
        c(0, "id"),
        c(2, "id"),
    );
    b.pair(
        children,
        placements,
        &[placements],
        vec![],
        c(0, "id"),
        c(0, "id"),
    );
    b.pair(
        children,
        occurrence,
        &[placements],
        vec![],
        c(0, "id"),
        c(0, "occurrence"),
    );
    b.pair(
        children,
        children,
        &[placements, placements],
        vec![
            eq(1, "parent", 0, "occurrence"),
            ScopePredicate::NotCode(c(0, "field"), SyntaxField::Body.code()),
        ],
        c(0, "id"),
        c(1, "id"),
    );
    let prefix = vec![
        ScopePredicate::Code(c(0, "syntax_kind"), SyntaxKind::StmtFunctionDef.code()),
        eq(1, "parent", 0, "id"),
        ScopePredicate::Code(c(1, "field"), SyntaxField::Body.code()),
        ScopePredicate::Integer(c(1, "ordinal"), 0),
    ];
    b.pair(
        occurrence,
        placements,
        &[occurrence, placements],
        prefix,
        c(0, "id"),
        c(1, "id"),
    );
    b.pair(
        placements,
        payload,
        &[placements, occurrence],
        vec![
            eq(1, "id", 0, "parent"),
            ScopePredicate::Code(c(1, "syntax_kind"), SyntaxKind::StmtFunctionDef.code()),
            ScopePredicate::Code(c(0, "field"), SyntaxField::Body.code()),
            ScopePredicate::Integer(c(0, "ordinal"), 0),
        ],
        c(0, "id"),
        c(0, "occurrence"),
    );
    b.pair(
        payload,
        occurrence,
        &[occurrence],
        vec![],
        c(0, "id"),
        c(0, "id"),
    );
    const NESTED: &[i16] = &[
        SyntaxKind::StmtFunctionDef as i16,
        SyntaxKind::StmtClassDef as i16,
        SyntaxKind::ExprLambda as i16,
    ];
    b.pair(
        payload,
        children,
        &[occurrence, placements],
        vec![
            eq(1, "parent", 0, "id"),
            ScopePredicate::CodeNotIn(c(0, "syntax_kind"), NESTED),
        ],
        c(0, "id"),
        c(1, "id"),
    );
    b.pair(
        children,
        payload,
        &[placements, occurrence],
        vec![
            eq(1, "id", 0, "parent"),
            ScopePredicate::CodeNotIn(c(1, "syntax_kind"), NESTED),
        ],
        c(0, "id"),
        c(0, "occurrence"),
    );
    let details = index(TypeId::of::<SyntaxDetailObservation>())?;
    let values = index(TypeId::of::<SyntaxDetail>())?;
    let literals = index(TypeId::of::<value::Literal>())?;
    b.pair(
        payload,
        details,
        &[occurrence, details],
        vec![eq(1, "occurrence", 0, "id")],
        c(0, "id"),
        c(1, "id"),
    );
    b.pair(
        payload,
        literals,
        &[occurrence, details, values, literals],
        vec![
            eq(1, "occurrence", 0, "id"),
            eq(2, "id", 1, "detail"),
            eq(3, "id", 2, "literal_literal"),
        ],
        c(0, "id"),
        c(3, "id"),
    );
    let uses = index(TypeId::of::<FlowUseObservation>())?;
    let regions = index(TypeId::of::<FlowRegionObservation>())?;
    let q = index(TypeId::of::<assertion::AssertionQualification>())?;
    b.pair(
        uses,
        regions,
        &[uses, q, regions, q],
        vec![
            eq(1, "id", 0, "qualification"),
            eq(2, "scope", 0, "scope"),
            eq(3, "id", 2, "qualification"),
            eq(3, "context", 1, "context"),
        ],
        c(0, "id"),
        c(2, "id"),
    );
    let bindings = index(TypeId::of::<BindingObservation>())?;
    let binding_events = index(TypeId::of::<BindingEvent>())?;
    b.pair(
        bindings,
        bindings,
        &[bindings, binding_events, bindings, binding_events],
        vec![
            eq(1, "id", 0, "event"),
            eq(2, "scope", 0, "scope"),
            eq(3, "id", 2, "event"),
            eq(3, "name", 1, "name"),
        ],
        c(0, "id"),
        c(2, "id"),
    );
    let references = index(TypeId::of::<ReferenceObservation>())?;
    let spellings = index(TypeId::of::<SyntaxObservation>())?;
    let scopes = index(TypeId::of::<LexicalScope>())?;
    b.pair(
        declarations,
        references,
        &[declarations, q, spellings, scopes, references, q],
        vec![
            eq(1, "id", 0, "qualification"),
            eq(2, "occurrence", 0, "name"),
            eq(3, "owner", 0, "parent"),
            eq(4, "name", 2, "spelling"),
            eq(4, "scope", 3, "id"),
            eq(5, "id", 4, "qualification"),
            eq(5, "context", 1, "context"),
        ],
        c(0, "id"),
        c(4, "id"),
    );
    let artifacts = index(TypeId::of::<SourceArtifact>())?;
    let scopes = index(TypeId::of::<CoverageScope>())?;
    let coverage = index(TypeId::of::<attribution::ProviderCoverage>())?;
    let modules = index(TypeId::of::<Module>())?;
    for field in ["input_input", "artifact_artifact"] {
        b.pair(
            artifacts,
            coverage,
            &[artifacts, scopes, coverage],
            vec![
                eq(
                    1,
                    field,
                    0,
                    if field == "input_input" {
                        "input"
                    } else {
                        "id"
                    },
                ),
                eq(2, "scope", 1, "id"),
            ],
            c(0, "id"),
            c(2, "id"),
        );
    }
    b.pair(
        artifacts,
        coverage,
        &[artifacts, modules, scopes, coverage],
        vec![
            eq(1, "source", 0, "id"),
            eq(2, "module_module", 1, "id"),
            eq(3, "scope", 2, "id"),
        ],
        c(0, "id"),
        c(3, "id"),
    );
    if let (Some(owner), Some(statement)) = (owner, statement) {
        use execution::source_call_records::{
            HeaderMember, SourceCallHeader, SourceFrameArgument, SourceFrameRelease,
            SourceInvocation,
        };
        own!(NormalizedCallEvent, "owner", OccurrenceOwnership);
        own!(SourceCallHeader, "event", NormalizedCallEvent);
        own!(HeaderMember, "header", SourceCallHeader);
        own!(SourceFrameRelease, "header", SourceCallHeader);
        own!(SourceFrameArgument, "release", SourceFrameRelease);
        own!(SourceInvocation, "release", SourceFrameRelease);
        let headers = index(TypeId::of::<SourceCallHeader>())?;
        let owners = index(TypeId::of::<OccurrenceOwnership>())?;
        let parameters = index(TypeId::of::<ParameterSyntaxObservation>())?;
        b.pair(
            headers,
            payload,
            &[headers, placements],
            vec![
                eq(1, "parent", 0, "declaration"),
                ScopePredicate::Code(c(1, "field"), SyntaxField::Body.code()),
            ],
            c(0, "id"),
            c(1, "occurrence"),
        );
        b.pair(
            payload,
            header,
            &[occurrence],
            vec![ScopePredicate::Code(
                c(0, "syntax_kind"),
                SyntaxKind::StmtFunctionDef.code(),
            )],
            c(0, "id"),
            c(0, "id"),
        );
        b.pair(
            header,
            parameters,
            &[occurrence, parameters],
            vec![eq(1, "function", 0, "id")],
            c(0, "id"),
            c(1, "id"),
        );
        b.pair(
            owner,
            header,
            &[refs, callables],
            vec![
                eq(1, "id", 0, "callable_callable"),
                ScopePredicate::IsNull(c(1, "source_declaration"), false),
            ],
            c(0, "id"),
            c(1, "source_declaration"),
        );
        b.pair(owner, refs, &[refs], vec![], c(0, "id"), c(0, "id"));
        b.pair(
            owner,
            payload,
            &[refs, owners],
            vec![eq(1, "entity", 0, "id")],
            c(0, "id"),
            c(1, "occurrence"),
        );
        b.pair(
            owner,
            callables,
            &[refs, callables],
            vec![eq(1, "id", 0, "callable_callable")],
            c(0, "id"),
            c(1, "id"),
        );
        b.pair(
            owner,
            payload,
            &[refs, callables, placements],
            vec![
                eq(1, "id", 0, "callable_callable"),
                eq(2, "parent", 1, "source_declaration"),
                ScopePredicate::Code(c(2, "field"), SyntaxField::Body.code()),
            ],
            c(0, "id"),
            c(2, "occurrence"),
        );
        b.pair(
            statement,
            payload,
            &[occurrence],
            vec![],
            c(0, "id"),
            c(0, "id"),
        );
    }
    b.finish()
}

/// A finite typed set expression over model-declared pair queries and the current grain.
/// Earlier node indices make the DAG bounded and its canonical order explicit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionNode {
    Query(usize),
    SelectedInput(usize),
    Union(usize, usize),
    Intersection(usize, usize),
    Difference(usize, usize),
    FilterSources { query: usize, sources: usize },
}
pub struct SelectionProgram {
    program: BudgetedCatalogProgram,
    nodes: Vec<SelectionNode>,
}
pub struct CompiledSelection {
    program: std::sync::Arc<CompiledScopeProgram>,
    nodes: Vec<SelectionNode>,
    bytes: Vec<u8>,
    identity: ContentHash,
    _charge: Box<dyn resources::Reservation>,
}
impl CompiledSelection {
    pub fn program(&self) -> &CompiledScopeProgram {
        &self.program
    }
    pub fn nodes(&self) -> &[SelectionNode] {
        &self.nodes
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn identity(&self) -> ContentHash {
        self.identity
    }
}
impl SelectionProgram {
    pub fn program(&self) -> &ScopeProgram {
        self.program.program()
    }
    pub fn nodes(&self) -> &[SelectionNode] {
        &self.nodes
    }
    pub fn compile(
        self,
        model: &ValidatedModel,
        budget: &resources::ResourceBudget,
        interner: Option<&std::sync::Mutex<ScopeInterner>>,
    ) -> Result<CompiledSelection, ModelError> {
        self.validate(model)?;
        let _copy = budget.reserve("source-selector-program-copy", self.program().allowance())?;
        let program = if let Some(interner) = interner {
            interner
                .lock()
                .map_err(|_| ModelError::Conflict("scope interner poisoned"))?
                .intern(self.program().clone(), model)?
        } else {
            ScopeInterner::new(budget)?.intern(self.program().clone(), model)?
        };
        let charge = budget.reserve(
            "source-selector-canonical-owner",
            program
                .canonical_bytes()
                .len()
                .saturating_add(self.nodes.capacity() * size_of::<SelectionNode>())
                .saturating_add(self.nodes.len() * 128 + 1024),
        )?;
        let mut sink = KeySink::recording(
            "source-selection/v1",
            program.canonical_bytes().len() + self.nodes.len() * 128 + 1024,
        );
        sink.part(b"program", program.canonical_bytes());
        for node in &self.nodes {
            let (tag, a, b) = match *node {
                SelectionNode::Query(q) => (b"query".as_slice(), q, None),
                SelectionNode::SelectedInput(i) => (b"selected".as_slice(), i, None),
                SelectionNode::Union(a, b) => (b"union".as_slice(), a, Some(b)),
                SelectionNode::Intersection(a, b) => (b"intersection".as_slice(), a, Some(b)),
                SelectionNode::Difference(a, b) => (b"difference".as_slice(), a, Some(b)),
                SelectionNode::FilterSources { query, sources } => {
                    (b"filter-sources".as_slice(), query, Some(sources))
                }
            };
            sink.part(b"operation", tag);
            sink.part(b"left", &(a as u64).to_le_bytes());
            if let Some(b) = b {
                sink.part(b"right", &(b as u64).to_le_bytes());
            }
        }
        let bytes = sink.recorded();
        let identity = ContentHash::of(&bytes);
        Ok(CompiledSelection {
            program,
            nodes: self.nodes,
            bytes,
            identity,
            _charge: charge,
        })
    }
    pub fn validate(&self, model: &ValidatedModel) -> Result<(), ModelError> {
        self.program().validate(model)?;
        let pair = |query: usize| -> Result<(usize, usize), ModelError> {
            match self.program().rules.get(query) {
                Some(
                    ScopeRule::Pairs { source, target, .. }
                    | ScopeRule::OptionalPairs { source, target, .. },
                ) => Ok((
                    self.program().ports[*source].input,
                    self.program().ports[*target].input,
                )),
                _ => Err(ModelError::Schema("source selection pair absent")),
            }
        };
        let mut outputs: Vec<usize> = Vec::with_capacity(self.nodes.len());
        for (ordinal, node) in self.nodes.iter().enumerate() {
            let prior = |n: usize| {
                outputs
                    .get(n)
                    .copied()
                    .filter(|_| n < ordinal)
                    .ok_or(ModelError::Schema("source selection DAG order"))
            };
            let output = match *node {
                SelectionNode::Query(q) => pair(q)?.1,
                SelectionNode::SelectedInput(i) => {
                    self.program()
                        .inputs
                        .get(i)
                        .ok_or(ModelError::Schema("source selection input absent"))?;
                    i
                }
                SelectionNode::Union(a, b)
                | SelectionNode::Intersection(a, b)
                | SelectionNode::Difference(a, b) => {
                    let a = prior(a)?;
                    if a != prior(b)? {
                        return Err(ModelError::Schema("source selection nominal set mismatch"));
                    }
                    a
                }
                SelectionNode::FilterSources { query, sources } => {
                    let (source, target) = pair(query)?;
                    if source != prior(sources)? {
                        return Err(ModelError::Schema(
                            "source selection source domain mismatch",
                        ));
                    }
                    target
                }
            };
            outputs.push(output);
        }
        if outputs.is_empty() {
            return Err(ModelError::Schema("source selection empty expression"));
        }
        Ok(())
    }
}
fn selector_finish(
    mut b: Builder,
    nodes: Vec<SelectionNode>,
) -> Result<SelectionProgram, ModelError> {
    b.reserve_retained(nodes.capacity() * size_of::<SelectionNode>() + nodes.len() * 128 + 1024)?;
    Ok(SelectionProgram {
        program: b.finish()?,
        nodes,
    })
}
fn typed_index<R: Record>(inputs: &[ValidationInput]) -> Result<usize, ModelError> {
    let mut indices = inputs
        .iter()
        .enumerate()
        .filter(|(_, i)| i.type_id() == TypeId::of::<R>());
    let index = indices
        .next()
        .ok_or(ModelError::Schema("SourceCall selector input"))?
        .0;
    if indices.next().is_some() {
        return Err(ModelError::Conflict(
            "SourceCall selector immutable input ambiguous",
        ));
    }
    Ok(index)
}
const SOURCE_ROLES: &[i16] = &[
    input::SourceRole::Release as i16,
    input::SourceRole::Example as i16,
    input::SourceRole::Test as i16,
    input::SourceRole::DocBlock as i16,
];
const PYTHON_SUFFIXES: &[&str] = &[".py", ".pyi"];
const STATEMENTS: &[i16] = &[
    SyntaxKind::StmtFunctionDef as i16,
    SyntaxKind::StmtClassDef as i16,
    SyntaxKind::StmtReturn as i16,
    SyntaxKind::StmtDelete as i16,
    SyntaxKind::StmtTypeAlias as i16,
    SyntaxKind::StmtAssign as i16,
    SyntaxKind::StmtAugAssign as i16,
    SyntaxKind::StmtAnnAssign as i16,
    SyntaxKind::StmtFor as i16,
    SyntaxKind::StmtWhile as i16,
    SyntaxKind::StmtIf as i16,
    SyntaxKind::StmtWith as i16,
    SyntaxKind::StmtMatch as i16,
    SyntaxKind::StmtRaise as i16,
    SyntaxKind::StmtTry as i16,
    SyntaxKind::StmtAssert as i16,
    SyntaxKind::StmtImport as i16,
    SyntaxKind::StmtImportFrom as i16,
    SyntaxKind::StmtGlobal as i16,
    SyntaxKind::StmtNonlocal as i16,
    SyntaxKind::StmtExpr as i16,
    SyntaxKind::StmtPass as i16,
    SyntaxKind::StmtBreak as i16,
    SyntaxKind::StmtContinue as i16,
    SyntaxKind::StmtIpyEscapeCommand as i16,
];
fn source_selected(artifact: usize, use_: usize) -> Vec<ScopePredicate> {
    vec![
        ScopePredicate::Parameter(c(artifact, "input"), 0),
        ScopePredicate::TextSuffixIn(c(artifact, "path"), PYTHON_SUFFIXES),
        eq(use_, "artifact", artifact, "id"),
        ScopePredicate::CodeIn(c(use_, "role"), SOURCE_ROLES),
    ]
}
#[derive(Clone, Copy)]
pub enum RootInventory {
    Events,
    Owners,
    UnownedStatements,
}
/// Input/context are slots 0/1. Missing event occurrence or source artifact remains a root;
/// owned/unowned inventories instead require actual supported Python source ownership.
pub fn root_inventory(
    inputs: Vec<ValidationInput>,
    input_relations: &[Relation],
    kind: RootInventory,
    budget: &resources::ResourceBudget,
) -> Result<SelectionProgram, ModelError> {
    let real = inputs.len();
    let occurrence = typed_index::<Occurrence>(&inputs)?;
    let artifacts = typed_index::<SourceArtifact>(&inputs)?;
    let uses = typed_index::<input::ArtifactUse>(&inputs)?;
    let events = typed_index::<NormalizedCallEvent>(&inputs)?;
    let mut b = Builder::new(inputs.clone(), real, input_relations, budget)?;
    match kind {
        RootInventory::Events => {
            let optional = vec![
                ScopeOptionalJoin {
                    row: 1,
                    keys: vec![(c(1, "id"), c(0, "site"))],
                },
                ScopeOptionalJoin {
                    row: 2,
                    keys: vec![(c(2, "id"), c(1, "source"))],
                },
            ];
            for missing in 1..=2 {
                b.program.rules.push(ScopeRule::OptionalPairs {
                    source: events,
                    target: events,
                    rows: vec![events, occurrence, artifacts],
                    optional: optional.clone(),
                    predicates: vec![
                        ScopePredicate::Parameter(c(0, "context"), 1),
                        ScopePredicate::IsNull(c(missing, "id"), true),
                    ],
                    source_key: c(0, "id"),
                    target_key: c(0, "id"),
                });
            }
            let mut predicates = source_selected(2, 3);
            predicates.extend([
                eq(1, "id", 0, "site"),
                eq(2, "id", 1, "source"),
                ScopePredicate::Parameter(c(0, "context"), 1),
            ]);
            b.pair(
                events,
                events,
                &[events, occurrence, artifacts, uses],
                predicates,
                c(0, "id"),
                c(0, "id"),
            );
            selector_finish(
                b,
                vec![
                    SelectionNode::Query(0),
                    SelectionNode::Query(1),
                    SelectionNode::Union(0, 1),
                    SelectionNode::Query(2),
                    SelectionNode::Union(2, 3),
                ],
            )
        }
        RootInventory::UnownedStatements => {
            let owners = typed_index::<OccurrenceOwnership>(&inputs)?;
            let mut predicates = source_selected(1, 2);
            predicates.extend([
                eq(1, "id", 0, "source"),
                ScopePredicate::CodeIn(c(0, "syntax_kind"), STATEMENTS),
                ScopePredicate::IsNull(c(3, "id"), true),
            ]);
            b.program.rules.push(ScopeRule::OptionalPairs {
                source: occurrence,
                target: occurrence,
                rows: vec![occurrence, artifacts, uses, owners],
                optional: vec![ScopeOptionalJoin {
                    row: 3,
                    keys: vec![(c(3, "occurrence"), c(0, "id"))],
                }],
                predicates,
                source_key: c(0, "id"),
                target_key: c(0, "id"),
            });
            selector_finish(b, vec![SelectionNode::Query(0)])
        }
        RootInventory::Owners => {
            let owners = typed_index::<OccurrenceOwnership>(&inputs)?;
            let refs = typed_index::<EntityRef>(&inputs)?;
            let callables = typed_index::<CallableEntity>(&inputs)?;
            let attempts = typed_index::<CallBindingAttempt>(&inputs)?;
            for names in [false, true] {
                let mut predicates = source_selected(2, 3);
                predicates.extend([
                    eq(1, "id", 0, "occurrence"),
                    eq(2, "id", 1, "source"),
                    if names {
                        ScopePredicate::Code(c(1, "syntax_kind"), SyntaxKind::ExprName.code())
                    } else {
                        ScopePredicate::CodeIn(c(1, "syntax_kind"), STATEMENTS)
                    },
                ]);
                b.pair(
                    refs,
                    refs,
                    &[owners, occurrence, artifacts, uses],
                    predicates,
                    c(0, "entity"),
                    c(0, "entity"),
                );
            }
            let mut predicates = source_selected(3, 4);
            predicates.extend([
                eq(1, "id", 0, "callable_callable"),
                eq(2, "id", 1, "source_declaration"),
                eq(3, "id", 2, "source"),
            ]);
            b.pair(
                refs,
                refs,
                &[refs, callables, occurrence, artifacts, uses],
                predicates,
                c(0, "id"),
                c(0, "id"),
            );
            let mut predicates = source_selected(4, 5);
            predicates.extend([
                eq(1, "id", 0, "event"),
                eq(2, "id", 1, "owner"),
                eq(3, "id", 1, "site"),
                eq(4, "id", 3, "source"),
                ScopePredicate::Parameter(c(1, "context"), 1),
            ]);
            b.pair(
                refs,
                refs,
                &[attempts, events, owners, occurrence, artifacts, uses],
                predicates,
                c(2, "entity"),
                c(2, "entity"),
            );
            selector_finish(
                b,
                vec![
                    SelectionNode::Query(0),
                    SelectionNode::Query(1),
                    SelectionNode::Union(0, 1),
                    SelectionNode::Query(2),
                    SelectionNode::Union(2, 3),
                    SelectionNode::Query(3),
                    SelectionNode::Union(4, 5),
                ],
            )
        }
    }
}
/// String payload projection code, from the append-only Literal codebook.
pub const OPAQUE_DOCSTRING_KIND: i16 = 3;
/// Docstrings use exact source/span containment, irrespective of structural path or role.
/// A missing/null docstring, observation, detail or literal produces no exclusion. Any selected
/// non-docstring occurrence sharing the content-addressed literal retains its rich payload.
pub fn docstring_payloads(
    inputs: Vec<ValidationInput>,
    input_relations: &[Relation],
    budget: &resources::ResourceBudget,
) -> Result<SelectionProgram, ModelError> {
    let occurrence = typed_index::<Occurrence>(&inputs)?;
    let declarations = typed_index::<DeclarationObservation>(&inputs)?;
    let details = typed_index::<SyntaxDetailObservation>(&inputs)?;
    let values = typed_index::<SyntaxDetail>(&inputs)?;
    let literals = typed_index::<value::Literal>(&inputs)?;
    let mut b = Builder::new(inputs.clone(), inputs.len(), input_relations, budget)?;
    b.pair(
        occurrence,
        occurrence,
        &[declarations, occurrence, occurrence],
        vec![
            eq(1, "id", 0, "docstring"),
            eq(2, "source", 1, "source"),
            ScopePredicate::SpanContains {
                outer_start: c(2, "start"),
                outer_end: c(2, "end"),
                inner_start: c(1, "start"),
                inner_end: c(1, "end"),
            },
            ScopePredicate::Code(c(2, "syntax_kind"), SyntaxKind::ExprStringLiteral.code()),
        ],
        c(2, "id"),
        c(2, "id"),
    );
    b.pair(
        occurrence,
        literals,
        &[occurrence, details, values],
        vec![
            eq(1, "occurrence", 0, "id"),
            eq(2, "id", 1, "detail"),
            ScopePredicate::IsNull(c(2, "literal_literal"), false),
        ],
        c(0, "id"),
        c(2, "literal_literal"),
    );
    b.pair(
        literals,
        literals,
        &[literals],
        vec![ScopePredicate::Code(c(0, "kind"), OPAQUE_DOCSTRING_KIND)],
        c(0, "id"),
        c(0, "id"),
    );
    selector_finish(
        b,
        vec![
            SelectionNode::Query(0),
            SelectionNode::SelectedInput(occurrence),
            SelectionNode::Difference(1, 0),
            SelectionNode::FilterSources {
                query: 1,
                sources: 2,
            },
            SelectionNode::FilterSources {
                query: 1,
                sources: 0,
            },
            SelectionNode::Difference(4, 3),
            SelectionNode::SelectedInput(literals),
            SelectionNode::Query(2),
            SelectionNode::Intersection(6, 7),
            SelectionNode::Intersection(5, 8),
        ],
    )
}

#[cfg(test)]
mod selector_controls {
    use super::*;
    #[test]
    fn source_selectors_validate_canonical_expression_and_absence_branches() {
        let model = model().unwrap();
        let budget = resources::ResourceBudget::fixed(64 << 20).unwrap();
        for enriched in [false, true] {
            let inputs = if enriched {
                execution::enriched_production::EnrichedData::inputs()
            } else {
                execution::source_call_records::SourceCallData::inputs()
            };
            let input_relations = inputs
                .iter()
                .map(|i| model.relation(i.name()).unwrap().clone())
                .collect::<Vec<_>>();
            let kinds = if enriched {
                vec![
                    RootInventory::Events,
                    RootInventory::Owners,
                    RootInventory::UnownedStatements,
                ]
            } else {
                vec![RootInventory::Events]
            };
            for kind in kinds {
                let selected =
                    root_inventory(inputs.clone(), &input_relations, kind, &budget).unwrap();
                selected.validate(&model).unwrap();
                if matches!(kind, RootInventory::Events) {
                    assert!(selected.program().rules.iter().take(2).all(|r|matches!(r,ScopeRule::OptionalPairs{predicates,..}if predicates.iter().any(|p|matches!(p,ScopePredicate::IsNull(_,true))))));
                }
                drop(selected.compile(&model, &budget, None).unwrap());
                assert_eq!(budget.reserved(), 0);
            }
            let a = docstring_payloads(inputs.clone(), &input_relations, &budget)
                .unwrap()
                .compile(&model, &budget, None)
                .unwrap();
            let mut changed = docstring_payloads(inputs, &input_relations, &budget).unwrap();
            changed.nodes[5] = SelectionNode::Union(4, 3);
            let b = changed.compile(&model, &budget, None).unwrap();
            assert_eq!(a.program().canonical_bytes(), b.program().canonical_bytes());
            assert_ne!(
                a.canonical_bytes(),
                b.canonical_bytes(),
                "set composition belongs to canonical selector identity"
            );
            assert_ne!(a.identity(), b.identity());
            drop((a, b));
            assert_eq!(budget.reserved(), 0);
        }
    }
}
